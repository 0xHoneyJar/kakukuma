pub mod batch;
pub mod chars;
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
use crate::cell::{parse_hex_color, Cell, Rgb};
use crate::export::ColorFormat;
use crate::import::{ImportOptions, FitMode, ImportColorMode};
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
        /// Color depth for ANSI output (auto-detects terminal support)
        #[arg(long, default_value = "auto")]
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
        output: Option<String>,
        /// Output file path (deprecated, use positional)
        #[arg(long = "output", hide = true)]
        output_flag: Option<String>,
        /// Export format (auto-detects from file extension when "auto")
        #[arg(long, default_value = "auto")]
        format: PreviewFormat,
        /// Color depth for ANSI output (auto-detects terminal support)
        #[arg(long, default_value = "auto")]
        color_format: CliColorFormat,
        /// Cell size for PNG export (WxH pixels)
        #[arg(long, default_value = "8x16")]
        cell_size: String,
        /// Integer scale factor for PNG export
        #[arg(long, default_value_t = 1)]
        scale: u32,
        /// Export full canvas (skip auto-crop)
        #[arg(long)]
        no_crop: bool,
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

    /// Undo last CLI operation.
    ///
    /// Uses a linear model: new operations discard redo history.
    /// Operations that overlap (e.g., clear over a drawn rect) store
    /// only the cleared state — undoing the clear restores the clear's
    /// snapshot, not the original drawn content.
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

    /// Resize canvas dimensions
    Resize {
        /// Path to .kaku file
        file: String,
        /// New width (8-128)
        #[arg(long)]
        width: Option<usize>,
        /// New height (8-128)
        #[arg(long)]
        height: Option<usize>,
        /// Canvas size as WxH (e.g., 32x24)
        #[arg(long, value_parser = parse_size)]
        size: Option<(usize, usize)>,
    },

    /// Clear canvas (reset all cells to default).
    ///
    /// Warning: clear is destructive. If clear overlaps with prior
    /// operations, undoing the clear may not fully restore all
    /// previous content. Consider exporting a backup first.
    Clear {
        /// Path to .kaku file
        file: String,
        /// Clear only a region (x1,y1,x2,y2)
        #[arg(long, value_parser = parse_region)]
        region: Option<(usize, usize, usize, usize)>,
    },

    /// Import image file onto canvas
    Import {
        /// Path to image file (PNG, JPEG, etc.)
        image: String,
        /// Path to output .kaku file
        output: Option<String>,
        /// Path to output .kaku file (deprecated, use positional)
        #[arg(long = "output", hide = true)]
        output_flag: Option<String>,
        /// Canvas width (8-128)
        #[arg(long, default_value_t = 48)]
        width: usize,
        /// Canvas height (8-128)
        #[arg(long, default_value_t = 32)]
        height: usize,
        /// Color quantization mode (default: truecolor stores full RGB)
        #[arg(long, default_value = "truecolor")]
        quantize: CliColorFormat,
        /// Color saturation boost (1.0=none, 2.0=double). Helps dark images survive 256-color palette.
        #[arg(long, default_value_t = 1.0)]
        boost: f32,
        /// Disable hue-preserving color matching (on by default)
        #[arg(long)]
        no_preserve_hue: bool,
        /// Disable brightness normalization (on by default)
        #[arg(long)]
        no_normalize: bool,
        /// Reduce to N distinct colors via k-means (2-64). Makes art cleaner and more readable.
        #[arg(long)]
        posterize: Option<usize>,
        /// Use mosaic mode: average each grid region instead of per-pixel sampling.
        #[arg(long)]
        mosaic: bool,
    },

    /// Convert an image directly to ANSI art on stdout (no intermediate file)
    Render {
        /// Path to image file (PNG, JPEG, etc.)
        image: String,
        /// Output width in characters
        #[arg(long, default_value_t = 48)]
        width: usize,
        /// Output height in cell rows
        #[arg(long, default_value_t = 24)]
        height: usize,
        /// Color format (auto-detects terminal support by default)
        #[arg(long, default_value = "auto")]
        color_format: CliColorFormat,
        /// Disable brightness normalization
        #[arg(long)]
        no_normalize: bool,
        /// Disable hue-preserving color matching
        #[arg(long)]
        no_preserve_hue: bool,
        /// Color saturation boost (1.0=none, 2.0=double)
        #[arg(long, default_value_t = 1.0)]
        boost: f32,
        /// Reduce to N distinct colors via k-means (2-64). Makes art cleaner and more readable.
        #[arg(long)]
        posterize: Option<usize>,
    },

    /// Palette management
    Palette {
        #[command(subcommand)]
        action: PaletteAction,
    },

    /// Execute batch operations from a JSON file
    Batch {
        /// Path to .kaku file
        file: String,
        /// Path to JSON commands file
        commands: String,
        /// Validate JSON without executing
        #[arg(long)]
        dry_run: bool,
    },

    /// List available block characters with metadata
    Chars {
        /// Filter by category (primary, shade, vertical-fill, horizontal-fill)
        #[arg(long)]
        category: Option<String>,
        /// Human-readable table output instead of JSON
        #[arg(long)]
        plain: bool,
    },

    /// Set or clear reference image for a project
    Reference {
        /// Path to .kaku file
        file: String,
        /// Path to reference image (PNG, JPEG, etc.)
        image: Option<String>,
        /// Clear reference image
        #[arg(long)]
        clear: bool,
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
    /// Block character: raw char (█) or name (full, shade-light, etc.). See 'kakukuma chars'.
    #[arg(long, name = "char")]
    pub ch: Option<String>,
    /// Apply symmetry
    #[arg(long, default_value = "off")]
    pub symmetry: CliSymmetry,
    /// Skip operation log (no undo for this operation)
    #[arg(long)]
    pub no_log: bool,
}

#[derive(ValueEnum, Clone, Debug, PartialEq)]
pub enum PreviewFormat {
    Auto,
    Ansi,
    Json,
    Plain,
    Png,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum CliColorFormat {
    /// Auto-detect terminal color support
    Auto,
    Truecolor,
    #[value(name = "256")]
    Color256,
    /// 256-color with hue-preserving quantization (better for dark/colorful images)
    #[value(name = "256-hue")]
    Color256Hue,
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
        /// Output file path
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
        CliColorFormat::Auto => ColorFormat::Auto,
        CliColorFormat::Truecolor => ColorFormat::TrueColor,
        CliColorFormat::Color256 => ColorFormat::Color256,
        CliColorFormat::Color256Hue => ColorFormat::Color256Hue,
        CliColorFormat::Color16 => ColorFormat::Color16,
    }
}

fn cli_error(msg: &str) -> ! {
    let json = serde_json::json!({
        "error": msg,
        "code": "USER_ERROR"
    });
    eprintln!("{}", json);
    std::process::exit(1)
}

fn internal_error(msg: &str) -> ! {
    let json = serde_json::json!({
        "error": msg,
        "code": "INTERNAL_ERROR"
    });
    eprintln!("{}", json);
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
        Command::Export { file, output, output_flag, format, color_format, cell_size, scale, no_crop } => {
            let out = output.or(output_flag)
                .unwrap_or_else(|| cli_error("Output path required. Usage: kakukuma export <FILE> <OUTPUT>"));
            preview::export_to_file(&file, &out, &format, &color_format, &cell_size, scale, no_crop)
        }
        Command::Resize { file, width, height, size } => {
            cmd_resize(&file, width, height, size)
        }
        Command::Clear { file, region } => cmd_clear(&file, region),
        Command::Import { image, output, output_flag, width, height, quantize, boost, no_preserve_hue, no_normalize, posterize, mosaic } => {
            let out = output.or(output_flag)
                .unwrap_or_else(|| cli_error("Output path required. Usage: kakukuma import <IMAGE> <OUTPUT>"));
            cmd_import(&image, &out, width, height, &quantize, boost, !no_preserve_hue, !no_normalize, posterize, mosaic)
        }
        Command::Render { image, width, height, color_format, no_normalize, no_preserve_hue, boost, posterize } => {
            cmd_render(&image, width, height, &color_format, !no_normalize, !no_preserve_hue, boost, posterize)
        }
        Command::Palette { action } => palette_cmd::run(action),
        Command::Batch { file, commands, dry_run } => batch::run_batch(&file, &commands, dry_run),
        Command::Chars { category, plain } => chars::run_chars(category.as_deref(), plain),
        Command::Reference { file, image, clear } => cmd_reference(&file, image.as_deref(), clear),
    }
}

fn cmd_resize(
    file: &str,
    width: Option<usize>,
    height: Option<usize>,
    size: Option<(usize, usize)>,
) -> io::Result<()> {
    let path = Path::new(file);
    let mut project = load_project(file);

    let (new_w, new_h) = match size {
        Some((w, h)) => (w, h),
        None => {
            let w = width.unwrap_or(project.canvas.width);
            let h = height.unwrap_or(project.canvas.height);
            (w, h)
        }
    };

    let old_w = project.canvas.width;
    let old_h = project.canvas.height;
    project.canvas.resize(new_w, new_h);
    let actual_w = project.canvas.width;
    let actual_h = project.canvas.height;
    let clamped = actual_w != new_w || actual_h != new_h;
    atomic_save(&mut project, path)?;

    let mut json = serde_json::json!({
        "resized": file,
        "old_width": old_w,
        "old_height": old_h,
        "new_width": actual_w,
        "new_height": actual_h,
    });
    if clamped {
        json["clamped"] = serde_json::json!(true);
        json["requested_width"] = serde_json::json!(new_w);
        json["requested_height"] = serde_json::json!(new_h);
    }
    println!("{}", serde_json::to_string(&json).unwrap());
    Ok(())
}

fn cmd_clear(file: &str, region: Option<(usize, usize, usize, usize)>) -> io::Result<()> {
    let path = Path::new(file);
    let mut project = load_project(file);

    let cleared = match region {
        Some((x1, y1, x2, y2)) => {
            let mut count = 0;
            for y in y1..=y2.min(project.canvas.height.saturating_sub(1)) {
                for x in x1..=x2.min(project.canvas.width.saturating_sub(1)) {
                    project.canvas.set(x, y, Cell::default());
                    count += 1;
                }
            }
            count
        }
        None => {
            let w = project.canvas.width;
            let h = project.canvas.height;
            for y in 0..h {
                for x in 0..w {
                    project.canvas.set(x, y, Cell::default());
                }
            }
            w * h
        }
    };

    atomic_save(&mut project, path)?;

    let json = serde_json::json!({
        "cleared": file,
        "cells_cleared": cleared,
        "region": region.map(|(x1,y1,x2,y2)| serde_json::json!({
            "x1": x1, "y1": y1, "x2": x2, "y2": y2
        })),
    });
    println!("{}", serde_json::to_string(&json).unwrap());
    Ok(())
}

fn cmd_import(
    image: &str,
    output: &str,
    width: usize,
    height: usize,
    quantize: &CliColorFormat,
    boost: f32,
    preserve_hue: bool,
    normalize: bool,
    posterize: Option<usize>,
    mosaic: bool,
) -> io::Result<()> {
    let img_path = Path::new(image);
    if !img_path.exists() {
        cli_error(&format!("Image not found: '{}'", image));
    }

    let out_path = Path::new(output);

    let color_mode = match quantize {
        CliColorFormat::Auto | CliColorFormat::Truecolor => ImportColorMode::TrueColor,
        CliColorFormat::Color256 | CliColorFormat::Color256Hue => ImportColorMode::Color256,
        CliColorFormat::Color16 => ImportColorMode::Color16,
    };

    let options = ImportOptions {
        fit_mode: FitMode::FitToCanvas,
        color_mode,
        color_boost: boost,
        preserve_hue,
        normalize,
        posterize,
        ..ImportOptions::default()
    };

    let w = width.clamp(crate::canvas::MIN_DIMENSION, crate::canvas::MAX_DIMENSION);
    let h = height.clamp(crate::canvas::MIN_DIMENSION, crate::canvas::MAX_DIMENSION);

    let cells = if mosaic {
        crate::import::import_mosaic(img_path, w, h, &options)
    } else {
        crate::import::import_image(img_path, w, h, &options)
    }
        .map_err(|e| {
            cli_error(&format!("Import failed: {}", e));
        })
        .unwrap();

    let mut canvas = Canvas::new_with_size(w, h);
    for (y, row) in cells.iter().enumerate() {
        for (x, cell) in row.iter().enumerate() {
            canvas.set(x, y, *cell);
        }
    }

    let mut project = Project::new(
        out_path.file_stem().and_then(|s| s.to_str()).unwrap_or("imported"),
        canvas,
        Rgb::WHITE,
        SymmetryMode::Off,
    );

    atomic_save(&mut project, out_path)?;

    let json = serde_json::json!({
        "imported": image,
        "output": output,
        "width": w,
        "height": h,
        "color_mode": format!("{:?}", color_mode),
    });
    println!("{}", serde_json::to_string(&json).unwrap());
    Ok(())
}

fn cmd_render(
    image: &str,
    width: usize,
    height: usize,
    color_format: &CliColorFormat,
    normalize: bool,
    preserve_hue: bool,
    boost: f32,
    posterize: Option<usize>,
) -> io::Result<()> {
    let img_path = Path::new(image);
    if !img_path.exists() {
        cli_error(&format!("Image not found: '{}'", image));
    }

    let options = ImportOptions {
        fit_mode: FitMode::FitToCanvas,
        color_mode: ImportColorMode::TrueColor,
        color_boost: boost,
        preserve_hue,
        normalize,
        posterize,
        ..ImportOptions::default()
    };

    let w = width.clamp(crate::canvas::MIN_DIMENSION, crate::canvas::MAX_DIMENSION);
    let h = height.clamp(crate::canvas::MIN_DIMENSION, crate::canvas::MAX_DIMENSION);

    let cells = crate::import::import_image(img_path, w, h, &options)
        .map_err(|e| {
            cli_error(&format!("Render failed: {}", e));
        })
        .unwrap();

    let mut canvas = Canvas::new_with_size(w, h);
    for (y, row) in cells.iter().enumerate() {
        for (x, cell) in row.iter().enumerate() {
            canvas.set(x, y, *cell);
        }
    }

    let cf = to_color_format(color_format);
    let resolved = crate::export::resolve_color_format(cf);
    let output = crate::export::to_ansi(&canvas, resolved);
    print!("{}", output);

    let cf_str = match resolved {
        ColorFormat::TrueColor => "truecolor",
        ColorFormat::Color256 => "256",
        ColorFormat::Color256Hue => "256-hue",
        ColorFormat::Color16 => "16",
        ColorFormat::Auto => "auto",
    };
    eprintln!("{}", serde_json::to_string(&serde_json::json!({
        "width": w,
        "height": h,
        "color_format": cf_str,
    })).unwrap());

    Ok(())
}

fn cmd_new(file: &str, width: usize, height: usize, force: bool) -> io::Result<()> {
    let path = Path::new(file);
    if path.exists() && !force {
        cli_error(&format!("'{}' already exists. Use --force to overwrite.", file));
    }

    let w = width.clamp(crate::canvas::MIN_DIMENSION, crate::canvas::MAX_DIMENSION);
    let h = height.clamp(crate::canvas::MIN_DIMENSION, crate::canvas::MAX_DIMENSION);
    let clamped = w != width || h != height;

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

    let mut json = serde_json::json!({
        "created": file,
        "width": w,
        "height": h,
    });
    if clamped {
        json["clamped"] = serde_json::json!(true);
        json["requested_width"] = serde_json::json!(width);
        json["requested_height"] = serde_json::json!(height);
    }
    println!("{}", serde_json::to_string(&json).unwrap());
    Ok(())
}

fn cmd_reference(file: &str, image: Option<&str>, clear: bool) -> io::Result<()> {
    let path = Path::new(file);
    let mut project = load_project(file);

    if clear {
        project.reference_image = None;
        atomic_save(&mut project, path)?;
        let json = serde_json::json!({
            "reference": serde_json::Value::Null,
            "file": file,
        });
        println!("{}", serde_json::to_string(&json).unwrap());
        return Ok(());
    }

    let img_path = match image {
        Some(p) => p,
        None => {
            cli_error("Provide an image path or use --clear to remove reference");
        }
    };

    // Validate image exists
    if !Path::new(img_path).exists() {
        cli_error(&format!("Image not found: '{}'", img_path));
    }

    // Store path relative to project file directory
    let project_dir = path.parent().unwrap_or(Path::new("."));
    let rel_path = match Path::new(img_path).strip_prefix(project_dir) {
        Ok(rel) => rel.to_string_lossy().to_string(),
        Err(_) => img_path.to_string(),
    };

    project.reference_image = Some(rel_path.clone());
    atomic_save(&mut project, path)?;

    let json = serde_json::json!({
        "reference": rel_path,
        "file": file,
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

    #[test]
    fn test_chars_command_parse() {
        let cli = Cli::try_parse_from(["kakukuma", "chars"]).unwrap();
        match cli.command.unwrap() {
            Command::Chars { category, plain } => {
                assert!(category.is_none());
                assert!(!plain);
            }
            _ => panic!("Expected Chars command"),
        }
    }

    #[test]
    fn test_chars_command_parse_with_category() {
        let cli = Cli::try_parse_from(["kakukuma", "chars", "--category", "shade"]).unwrap();
        match cli.command.unwrap() {
            Command::Chars { category, plain } => {
                assert_eq!(category.as_deref(), Some("shade"));
                assert!(!plain);
            }
            _ => panic!("Expected Chars command"),
        }
    }

    #[test]
    fn test_chars_command_parse_plain() {
        let cli = Cli::try_parse_from(["kakukuma", "chars", "--plain"]).unwrap();
        match cli.command.unwrap() {
            Command::Chars { category, plain } => {
                assert!(category.is_none());
                assert!(plain);
            }
            _ => panic!("Expected Chars command"),
        }
    }
}
