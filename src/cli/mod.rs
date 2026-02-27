pub mod draw;
pub mod preview;
pub mod inspect;
pub mod diff;
pub mod stats;
pub mod history_cmd;
pub mod palette_cmd;

use std::io;
use std::path::Path;

use clap::{Parser, Subcommand, ValueEnum};

use crate::canvas::Canvas;
use crate::cell::{parse_hex_color, Rgb};
use crate::export::ColorFormat;
use crate::project::Project;
use crate::symmetry::SymmetryMode;

#[derive(Parser)]
#[command(name = "kakukuma", about = "Terminal ANSI art editor")]
pub struct Cli {
    /// Open .kaku file in TUI editor
    pub file: Option<String>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Create a new .kaku project file
    New {
        /// Path for the new .kaku file
        file: String,
        /// Canvas width (8-128)
        #[arg(long, default_value_t = 48)]
        width: usize,
        /// Canvas height (8-128)
        #[arg(long, default_value_t = 32)]
        height: usize,
        /// Canvas size as WxH (e.g., 32x24)
        #[arg(long, value_parser = parse_size)]
        size: Option<(usize, usize)>,
        /// Overwrite existing file
        #[arg(long)]
        force: bool,
    },

    /// Draw on canvas using a tool
    Draw {
        #[command(subcommand)]
        tool: DrawTool,
    },

    /// Render canvas to stdout
    Preview {
        /// Path to .kaku file
        file: String,
        /// Output format
        #[arg(long, default_value = "ansi")]
        format: PreviewFormat,
        /// Preview subregion (x1,y1,x2,y2)
        #[arg(long, value_parser = parse_region)]
        region: Option<(usize, usize, usize, usize)>,
        /// Color depth for ANSI output
        #[arg(long, default_value = "truecolor")]
        color_format: CliColorFormat,
    },

    /// Query canvas cell data
    Inspect {
        /// Path to .kaku file
        file: String,
        /// Cell coordinate (x,y)
        #[arg(value_parser = parse_coord)]
        coord: Option<(usize, usize)>,
        /// Inspect region (x1,y1,x2,y2)
        #[arg(long, value_parser = parse_region)]
        region: Option<(usize, usize, usize, usize)>,
        /// Inspect entire row
        #[arg(long)]
        row: Option<usize>,
        /// Inspect entire column
        #[arg(long)]
        col: Option<usize>,
    },

    /// Export canvas to file
    Export {
        /// Path to .kaku file
        file: String,
        /// Output file path
        #[arg(long)]
        output: String,
        /// Export format
        #[arg(long, default_value = "ansi")]
        format: PreviewFormat,
        /// Color depth for ANSI output
        #[arg(long, default_value = "truecolor")]
        color_format: CliColorFormat,
    },

    /// Compare two canvas files
    Diff {
        /// First .kaku file
        file1: String,
        /// Second .kaku file (omit for --before mode)
        file2: Option<String>,
        /// Compare current state vs before last operation
        #[arg(long)]
        before: bool,
    },

    /// Canvas statistics
    Stats {
        /// Path to .kaku file
        file: String,
    },

    /// Undo last CLI operation
    Undo {
        /// Path to .kaku file
        file: String,
        /// Number of operations to undo
        #[arg(long, default_value_t = 1)]
        count: usize,
    },

    /// Redo last undone CLI operation
    Redo {
        /// Path to .kaku file
        file: String,
        /// Number of operations to redo
        #[arg(long, default_value_t = 1)]
        count: usize,
    },

    /// Show operation log
    History {
        /// Path to .kaku file
        file: String,
        /// Show full mutation details
        #[arg(long)]
        full: bool,
    },

    /// Palette management
    Palette {
        #[command(subcommand)]
        action: PaletteAction,
    },
}

#[derive(Subcommand)]
pub enum DrawTool {
    /// Draw a single cell
    Pencil {
        /// Path to .kaku file
        file: String,
        /// Cell coordinate (x,y)
        #[arg(value_parser = parse_coord)]
        coord: (usize, usize),
        #[command(flatten)]
        opts: DrawOpts,
    },
    /// Erase a cell
    Eraser {
        /// Path to .kaku file
        file: String,
        /// Cell coordinate (x,y)
        #[arg(value_parser = parse_coord)]
        coord: (usize, usize),
        /// Erase region (x1,y1,x2,y2)
        #[arg(long, value_parser = parse_region)]
        region: Option<(usize, usize, usize, usize)>,
    },
    /// Draw a line between two points
    Line {
        /// Path to .kaku file
        file: String,
        /// Start coordinate (x,y)
        #[arg(value_parser = parse_coord)]
        from: (usize, usize),
        /// End coordinate (x,y)
        #[arg(value_parser = parse_coord)]
        to: (usize, usize),
        #[command(flatten)]
        opts: DrawOpts,
    },
    /// Draw a rectangle
    Rect {
        /// Path to .kaku file
        file: String,
        /// Top-left coordinate (x,y)
        #[arg(value_parser = parse_coord)]
        from: (usize, usize),
        /// Bottom-right coordinate (x,y)
        #[arg(value_parser = parse_coord)]
        to: (usize, usize),
        /// Fill the rectangle
        #[arg(long)]
        filled: bool,
        #[command(flatten)]
        opts: DrawOpts,
    },
    /// Flood fill from a point
    Fill {
        /// Path to .kaku file
        file: String,
        /// Start coordinate (x,y)
        #[arg(value_parser = parse_coord)]
        coord: (usize, usize),
        #[command(flatten)]
        opts: DrawOpts,
    },
    /// Pick color from a cell
    Eyedropper {
        /// Path to .kaku file
        file: String,
        /// Cell coordinate (x,y)
        #[arg(value_parser = parse_coord)]
        coord: (usize, usize),
    },
}

#[derive(clap::Args)]
pub struct DrawOpts {
    /// Set foreground color (hex, e.g., "#FF0000")
    #[arg(long)]
    pub color: Option<String>,
    /// Set foreground explicitly
    #[arg(long)]
    pub fg: Option<String>,
    /// Set background explicitly
    #[arg(long)]
    pub bg: Option<String>,
    /// Block character to use
    #[arg(long, name = "char")]
    pub ch: Option<char>,
    /// Apply symmetry
    #[arg(long, default_value = "off")]
    pub symmetry: CliSymmetry,
    /// Skip operation log (no undo for this operation)
    #[arg(long)]
    pub no_log: bool,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum PreviewFormat {
    Ansi,
    Json,
    Plain,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum CliColorFormat {
    Truecolor,
    #[value(name = "256")]
    Color256,
    #[value(name = "16")]
    Color16,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum CliSymmetry {
    Off,
    Horizontal,
    Vertical,
    Quad,
}

#[derive(Subcommand)]
pub enum PaletteAction {
    /// List available .palette files
    List,
    /// Show colors in a palette
    Show { name: String },
    /// Create palette from canvas colors
    Create { name: String, file: String },
    /// Export palette to file
    Export {
        name: String,
        #[arg(long)]
        output: String,
    },
    /// Add color to palette
    Add { name: String, color: String },
    /// List available themes
    Themes,
    /// Show colors in a theme
    Theme { name: String },
}

// --- Parsers ---

pub fn parse_coord(s: &str) -> Result<(usize, usize), String> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 2 {
        return Err(format!("Expected X,Y format, got '{}'", s));
    }
    let x = parts[0].trim().parse::<usize>()
        .map_err(|_| format!("Invalid X coordinate: '{}'", parts[0]))?;
    let y = parts[1].trim().parse::<usize>()
        .map_err(|_| format!("Invalid Y coordinate: '{}'", parts[1]))?;
    Ok((x, y))
}

pub fn parse_region(s: &str) -> Result<(usize, usize, usize, usize), String> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 4 {
        return Err(format!("Expected X1,Y1,X2,Y2 format, got '{}'", s));
    }
    let x1 = parts[0].trim().parse::<usize>()
        .map_err(|_| format!("Invalid X1: '{}'", parts[0]))?;
    let y1 = parts[1].trim().parse::<usize>()
        .map_err(|_| format!("Invalid Y1: '{}'", parts[1]))?;
    let x2 = parts[2].trim().parse::<usize>()
        .map_err(|_| format!("Invalid X2: '{}'", parts[2]))?;
    let y2 = parts[3].trim().parse::<usize>()
        .map_err(|_| format!("Invalid Y2: '{}'", parts[3]))?;
    Ok((x1, y1, x2, y2))
}

pub fn parse_size(s: &str) -> Result<(usize, usize), String> {
    let parts: Vec<&str> = s.split('x').collect();
    if parts.len() != 2 {
        return Err(format!("Expected WxH format (e.g., 32x24), got '{}'", s));
    }
    let w = parts[0].trim().parse::<usize>()
        .map_err(|_| format!("Invalid width: '{}'", parts[0]))?;
    let h = parts[1].trim().parse::<usize>()
        .map_err(|_| format!("Invalid height: '{}'", parts[1]))?;
    Ok((w, h))
}

// --- Helpers ---

pub fn resolve_colors(opts: &DrawOpts) -> (Option<Rgb>, Option<Rgb>) {
    let fg_str = opts.fg.as_deref().or(opts.color.as_deref());
    let fg = match fg_str {
        Some(s) => match parse_hex_color(s) {
            Some(c) => Some(c),
            None => cli_error(&format!(
                "Invalid hex color '{}'. Expected format: #RRGGBB (e.g. #FF0000)", s
            )),
        },
        None => Some(Rgb::WHITE),
    };
    let bg = match opts.bg.as_deref() {
        Some(s) => match parse_hex_color(s) {
            Some(c) => Some(c),
            None => cli_error(&format!(
                "Invalid hex color '{}'. Expected format: #RRGGBB (e.g. #FF0000)", s
            )),
        },
        None => None,
    };
    (fg, bg)
}

pub fn to_symmetry_mode(s: &CliSymmetry) -> SymmetryMode {
    match s {
        CliSymmetry::Off => SymmetryMode::Off,
        CliSymmetry::Horizontal => SymmetryMode::Horizontal,
        CliSymmetry::Vertical => SymmetryMode::Vertical,
        CliSymmetry::Quad => SymmetryMode::Quad,
    }
}

pub fn to_color_format(f: &CliColorFormat) -> ColorFormat {
    match f {
        CliColorFormat::Truecolor => ColorFormat::TrueColor,
        CliColorFormat::Color256 => ColorFormat::Color256,
        CliColorFormat::Color16 => ColorFormat::Color16,
    }
}

fn cli_error(msg: &str) -> ! {
    eprintln!("Error: {}", msg);
    std::process::exit(1)
}

fn internal_error(msg: &str) -> ! {
    eprintln!("Internal error: {}", msg);
    std::process::exit(2)
}

fn load_project(path: &str) -> Project {
    let p = Path::new(path);
    if !p.exists() {
        cli_error(&format!("File not found: '{}'", path));
    }
    Project::load_from_file(p).unwrap_or_else(|e| {
        internal_error(&format!("Failed to load '{}': {}", path, e));
    })
}

fn atomic_save(project: &mut Project, path: &Path) -> io::Result<()> {
    let tmp = path.with_extension("kaku.tmp");
    project.save_to_file(&tmp)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    std::fs::rename(&tmp, path)
}

/// Route a CLI command to the appropriate handler.
pub fn run(cmd: Command) -> io::Result<()> {
    match cmd {
        Command::New { file, width, height, size, force } => {
            let (w, h) = size.unwrap_or((width, height));
            cmd_new(&file, w, h, force)
        }
        Command::Draw { tool } => draw::run(tool),
        Command::Preview { file, format, region, color_format } => {
            preview::run(&file, &format, region, &color_format)
        }
        Command::Inspect { file, coord, region, row, col } => {
            inspect::run(&file, coord, region, row, col)
        }
        Command::Diff { file1, file2, before } => {
            diff::run(&file1, file2.as_deref(), before)
        }
        Command::Stats { file } => stats::run(&file),
        Command::Undo { file, count } => history_cmd::undo(&file, count),
        Command::Redo { file, count } => history_cmd::redo(&file, count),
        Command::History { file, full } => history_cmd::history(&file, full),
        Command::Export { file, output, format, color_format } => {
            preview::export_to_file(&file, &output, &format, &color_format)
        }
        Command::Palette { action } => palette_cmd::run(action),
    }
}

fn cmd_new(file: &str, width: usize, height: usize, force: bool) -> io::Result<()> {
    let path = Path::new(file);
    if path.exists() && !force {
        cli_error(&format!("'{}' already exists. Use --force to overwrite.", file));
    }

    let w = width.clamp(crate::canvas::MIN_DIMENSION, crate::canvas::MAX_DIMENSION);
    let h = height.clamp(crate::canvas::MIN_DIMENSION, crate::canvas::MAX_DIMENSION);

    let canvas = Canvas::new_with_size(w, h);
    let mut project = Project::new(
        path.file_stem().and_then(|s| s.to_str()).unwrap_or("untitled"),
        canvas,
        Rgb::WHITE,
        SymmetryMode::Off,
    );

    project.save_to_file(path)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    // Initialize empty log
    let log = crate::oplog::log_path(path);
    crate::oplog::init_log(&log)?;

    let json = serde_json::json!({
        "created": file,
        "width": w,
        "height": h,
    });
    println!("{}", serde_json::to_string(&json).unwrap());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_coord_valid() {
        assert_eq!(parse_coord("5,5"), Ok((5, 5)));
        assert_eq!(parse_coord("0,0"), Ok((0, 0)));
        assert_eq!(parse_coord("127,127"), Ok((127, 127)));
    }

    #[test]
    fn test_parse_coord_with_spaces() {
        assert_eq!(parse_coord(" 5 , 5 "), Ok((5, 5)));
    }

    #[test]
    fn test_parse_coord_invalid() {
        assert!(parse_coord("abc").is_err());
        assert!(parse_coord("5").is_err());
        assert!(parse_coord("5,5,5").is_err());
        assert!(parse_coord("a,b").is_err());
    }

    #[test]
    fn test_parse_region_valid() {
        assert_eq!(parse_region("0,0,10,10"), Ok((0, 0, 10, 10)));
    }

    #[test]
    fn test_parse_region_invalid() {
        assert!(parse_region("0,0,10").is_err());
        assert!(parse_region("abc").is_err());
    }

    #[test]
    fn test_parse_size_valid() {
        assert_eq!(parse_size("32x24"), Ok((32, 24)));
        assert_eq!(parse_size("8x8"), Ok((8, 8)));
    }

    #[test]
    fn test_parse_size_invalid() {
        assert!(parse_size("32").is_err());
        assert!(parse_size("32,24").is_err());
        assert!(parse_size("axb").is_err());
    }

    #[test]
    fn test_resolve_colors_default() {
        let opts = DrawOpts {
            color: None, fg: None, bg: None,
            ch: None, symmetry: CliSymmetry::Off, no_log: false,
        };
        let (fg, bg) = resolve_colors(&opts);
        assert_eq!(fg, Some(Rgb::WHITE));
        assert_eq!(bg, None);
    }

    #[test]
    fn test_resolve_colors_with_color() {
        let opts = DrawOpts {
            color: Some("#FF0000".to_string()), fg: None, bg: None,
            ch: None, symmetry: CliSymmetry::Off, no_log: false,
        };
        let (fg, bg) = resolve_colors(&opts);
        assert_eq!(fg, Some(Rgb::new(255, 0, 0)));
        assert_eq!(bg, None);
    }

    #[test]
    fn test_resolve_colors_fg_overrides_color() {
        let opts = DrawOpts {
            color: Some("#FF0000".to_string()),
            fg: Some("#00FF00".to_string()),
            bg: Some("#0000FF".to_string()),
            ch: None, symmetry: CliSymmetry::Off, no_log: false,
        };
        let (fg, bg) = resolve_colors(&opts);
        assert_eq!(fg, Some(Rgb::new(0, 255, 0)));
        assert_eq!(bg, Some(Rgb::new(0, 0, 255)));
    }

    #[test]
    fn test_symmetry_mode_mapping() {
        assert_eq!(to_symmetry_mode(&CliSymmetry::Off), SymmetryMode::Off);
        assert_eq!(to_symmetry_mode(&CliSymmetry::Horizontal), SymmetryMode::Horizontal);
        assert_eq!(to_symmetry_mode(&CliSymmetry::Vertical), SymmetryMode::Vertical);
        assert_eq!(to_symmetry_mode(&CliSymmetry::Quad), SymmetryMode::Quad);
    }
}
