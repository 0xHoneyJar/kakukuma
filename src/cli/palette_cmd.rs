use std::io;
use std::path::Path;

use crate::cell::{parse_hex_color, Rgb};
use crate::cli::{load_project, PaletteAction};
use crate::palette::{self, CustomPalette, DEFAULT_PALETTE};
use crate::theme::THEMES;

pub fn run(action: PaletteAction) -> io::Result<()> {
    match action {
        PaletteAction::List => cmd_list(),
        PaletteAction::Show { name } => cmd_show(&name),
        PaletteAction::Create { name, file } => cmd_create(&name, &file),
        PaletteAction::Export { name, output } => cmd_export(&name, &output),
        PaletteAction::Add { name, color } => cmd_add(&name, &color),
        PaletteAction::Themes => cmd_themes(),
        PaletteAction::Theme { name } => cmd_theme(&name),
    }
}

fn palette_dir() -> std::path::PathBuf {
    std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
}

fn cmd_list() -> io::Result<()> {
    let dir = palette_dir();
    let files = palette::list_palette_files(&dir);

    let default_colors: Vec<_> = DEFAULT_PALETTE.iter()
        .map(|c| serde_json::json!(c.name()))
        .collect();

    let json = serde_json::json!({
        "default_palette": {
            "name": "default",
            "count": DEFAULT_PALETTE.len(),
            "colors": default_colors,
        },
        "custom_palettes": files,
    });
    println!("{}", serde_json::to_string_pretty(&json).unwrap());
    Ok(())
}

fn cmd_show(name: &str) -> io::Result<()> {
    if name == "default" {
        let colors: Vec<_> = DEFAULT_PALETTE.iter()
            .map(|c| serde_json::json!({"hex": c.name(), "r": c.r, "g": c.g, "b": c.b}))
            .collect();
        let json = serde_json::json!({
            "name": "default",
            "count": DEFAULT_PALETTE.len(),
            "colors": colors,
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
        return Ok(());
    }

    let path = palette_dir().join(format!("{}.palette", name));
    match palette::load_palette(&path) {
        Ok(pal) => {
            let colors: Vec<_> = pal.colors.iter()
                .map(|c| serde_json::json!({"hex": c.name(), "r": c.r, "g": c.g, "b": c.b}))
                .collect();
            let json = serde_json::json!({
                "name": pal.name,
                "count": pal.colors.len(),
                "colors": colors,
            });
            println!("{}", serde_json::to_string_pretty(&json).unwrap());
            Ok(())
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_create(name: &str, file: &str) -> io::Result<()> {
    let project = load_project(file);
    let canvas = &project.canvas;

    // Extract unique colors from canvas
    let mut colors = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for y in 0..canvas.height {
        for x in 0..canvas.width {
            if let Some(cell) = canvas.get(x, y) {
                if let Some(fg) = cell.fg {
                    if seen.insert((fg.r, fg.g, fg.b)) {
                        colors.push(fg);
                    }
                }
                if let Some(bg) = cell.bg {
                    if seen.insert((bg.r, bg.g, bg.b)) {
                        colors.push(bg);
                    }
                }
            }
        }
    }

    let pal = CustomPalette {
        name: name.to_string(),
        colors: colors.clone(),
    };

    let path = palette_dir().join(format!("{}.palette", name));
    palette::save_palette(&pal, &path)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let json = serde_json::json!({
        "created": format!("{}.palette", name),
        "name": name,
        "colors_extracted": colors.len(),
    });
    println!("{}", serde_json::to_string(&json).unwrap());
    Ok(())
}

fn cmd_export(name: &str, output: &str) -> io::Result<()> {
    let src = palette_dir().join(format!("{}.palette", name));
    if !src.exists() {
        eprintln!("Error: Palette '{}' not found", name);
        std::process::exit(1);
    }
    std::fs::copy(&src, Path::new(output))?;

    let json = serde_json::json!({
        "exported": output,
        "source": format!("{}.palette", name),
    });
    println!("{}", serde_json::to_string(&json).unwrap());
    Ok(())
}

fn cmd_add(name: &str, color: &str) -> io::Result<()> {
    let rgb = match parse_hex_color(color) {
        Some(c) => c,
        None => {
            eprintln!("Error: Invalid hex color '{}'", color);
            std::process::exit(1);
        }
    };

    let path = palette_dir().join(format!("{}.palette", name));
    let mut pal = if path.exists() {
        palette::load_palette(&path)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
    } else {
        CustomPalette {
            name: name.to_string(),
            colors: Vec::new(),
        }
    };

    pal.colors.push(rgb);
    palette::save_palette(&pal, &path)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let json = serde_json::json!({
        "ok": true,
        "palette": name,
        "added": rgb.name(),
        "total_colors": pal.colors.len(),
    });
    println!("{}", serde_json::to_string(&json).unwrap());
    Ok(())
}

fn cmd_themes() -> io::Result<()> {
    let themes: Vec<_> = THEMES.iter().map(|t| {
        serde_json::json!({"name": t.name})
    }).collect();

    let json = serde_json::json!({
        "themes": themes,
        "count": THEMES.len(),
    });
    println!("{}", serde_json::to_string_pretty(&json).unwrap());
    Ok(())
}

fn cmd_theme(name: &str) -> io::Result<()> {
    let theme = THEMES.iter().find(|t| t.name.eq_ignore_ascii_case(name));
    match theme {
        Some(t) => {
            let json = serde_json::json!({
                "name": t.name,
                "border_accent": format_color(t.border_accent),
                "header_bg": format_color(t.header_bg),
                "highlight": format_color(t.highlight),
                "accent": format_color(t.accent),
                "dim": format_color(t.dim),
                "separator": format_color(t.separator),
                "panel_bg": format_color(t.panel_bg),
                "grid_even": format_color(t.grid_even),
                "grid_odd": format_color(t.grid_odd),
            });
            println!("{}", serde_json::to_string_pretty(&json).unwrap());
            Ok(())
        }
        None => {
            eprintln!("Error: Theme '{}' not found. Available: {}", name,
                THEMES.iter().map(|t| t.name).collect::<Vec<_>>().join(", "));
            std::process::exit(1);
        }
    }
}

fn format_color(color: ratatui::style::Color) -> serde_json::Value {
    match color {
        ratatui::style::Color::Indexed(idx) => {
            let rgb = crate::cell::color256_to_rgb(idx);
            serde_json::json!({"index": idx, "hex": rgb.name()})
        }
        ratatui::style::Color::Rgb(r, g, b) => {
            serde_json::json!({"hex": Rgb::new(r, g, b).name()})
        }
        _ => serde_json::json!(format!("{:?}", color)),
    }
}
