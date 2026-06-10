use std::io;
use std::path::Path;

use crate::cli::{CliColorFormat, PreviewFormat, load_project, to_color_format};
use crate::export;

pub fn run(
    file: &str,
    format: &PreviewFormat,
    region: Option<(usize, usize, usize, usize)>,
    color_format: &CliColorFormat,
) -> io::Result<()> {
    let project = load_project(file);
    let cf = to_color_format(color_format);

    match format {
        PreviewFormat::Ansi | PreviewFormat::Auto => {
            let output = if let Some((x1, y1, x2, y2)) = region {
                ansi_region(&project, x1, y1, x2, y2, cf)
            } else {
                export::to_ansi(&project.canvas, cf)
            };
            print!("{}", output);
            Ok(())
        }
        PreviewFormat::Json => {
            let output = json_preview(&project, region);
            println!("{}", output);
            Ok(())
        }
        PreviewFormat::Plain => {
            let output = if let Some((x1, y1, x2, y2)) = region {
                plain_region(&project, x1, y1, x2, y2)
            } else {
                export::to_plain_text(&project.canvas)
            };
            print!("{}", output);
            Ok(())
        }
        PreviewFormat::Png => {
            eprintln!("{{\"error\":\"PNG format not supported for preview (stdout). Use 'export' instead.\",\"code\":\"USER_ERROR\"}}");
            std::process::exit(1);
        }
    }
}

/// Detect export format from output file extension when format is Auto.
fn detect_format(output: &str, explicit: &PreviewFormat) -> PreviewFormat {
    if *explicit != PreviewFormat::Auto {
        return explicit.clone();
    }
    match Path::new(output).extension().and_then(|e| e.to_str()) {
        Some("png") => PreviewFormat::Png,
        Some("json") => PreviewFormat::Json,
        Some("txt") => PreviewFormat::Plain,
        _ => PreviewFormat::Ansi,
    }
}

/// Parse cell size string like "8x16" into (width, height).
fn parse_cell_size(s: &str) -> Result<(u32, u32), String> {
    let parts: Vec<&str> = s.split('x').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid cell size '{}', expected WxH (e.g., 8x16)", s));
    }
    let w = parts[0].parse::<u32>().map_err(|_| format!("Invalid width in '{}'", s))?;
    let h = parts[1].parse::<u32>().map_err(|_| format!("Invalid height in '{}'", s))?;
    if w == 0 || h == 0 || w > 64 || h > 64 {
        return Err(format!("Cell size {}x{} out of range (1-64)", w, h));
    }
    Ok((w, h))
}

pub fn export_to_file(
    file: &str,
    output: &str,
    format: &PreviewFormat,
    color_format: &CliColorFormat,
    cell_size: &str,
    scale: u32,
    no_crop: bool,
) -> io::Result<()> {
    let project = load_project(file);
    let cf = to_color_format(color_format);
    let resolved_format = detect_format(output, format);

    match resolved_format {
        PreviewFormat::Png => {
            let (cw, ch) = parse_cell_size(cell_size).map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidInput, e)
            })?;
            let img = export::to_png(&project.canvas, cw, ch, scale, !no_crop);
            let (w, h) = (img.width(), img.height());
            img.save(output).map_err(|e| {
                io::Error::new(io::ErrorKind::Other, format!("PNG save failed: {}", e))
            })?;
            let json = serde_json::json!({
                "exported": output,
                "format": "png",
                "width": w,
                "height": h,
                "cell_size": format!("{}x{}", cw, ch),
            });
            println!("{}", serde_json::to_string(&json).unwrap());
        }
        _ => {
            let content = match resolved_format {
                PreviewFormat::Ansi | PreviewFormat::Auto => export::to_ansi(&project.canvas, cf),
                PreviewFormat::Plain => export::to_plain_text(&project.canvas),
                PreviewFormat::Json => json_preview(&project, None),
                PreviewFormat::Png => unreachable!(),
            };

            std::fs::write(output, &content)?;

            let format_str = match resolved_format {
                PreviewFormat::Ansi | PreviewFormat::Auto => "ansi",
                PreviewFormat::Plain => "plain",
                PreviewFormat::Json => "json",
                PreviewFormat::Png => unreachable!(),
            };
            let cf_str = match color_format {
                CliColorFormat::Auto => "auto",
                CliColorFormat::Truecolor => "truecolor",
                CliColorFormat::Color256 => "256",
                CliColorFormat::Color256Hue => "256-hue",
                CliColorFormat::Color16 => "16",
            };

            let json = serde_json::json!({
                "exported": output,
                "format": format_str,
                "color_format": cf_str,
            });
            println!("{}", serde_json::to_string(&json).unwrap());
        }
    }
    Ok(())
}

fn json_preview(project: &crate::project::Project, region: Option<(usize, usize, usize, usize)>) -> String {
    let canvas = &project.canvas;
    let (x_start, y_start, x_end, y_end) = region
        .unwrap_or((0, 0, canvas.width.saturating_sub(1), canvas.height.saturating_sub(1)));

    let x_end = x_end.min(canvas.width.saturating_sub(1));
    let y_end = y_end.min(canvas.height.saturating_sub(1));

    let mut cells = Vec::new();
    let mut non_empty_count = 0;

    for y in y_start..=y_end {
        let mut row = Vec::new();
        for x in x_start..=x_end {
            if let Some(cell) = canvas.get(x, y) {
                if !cell.is_empty() {
                    non_empty_count += 1;
                }
                row.push(serde_json::json!({
                    "x": x,
                    "y": y,
                    "fg": cell.fg.map(|c| c.name()),
                    "bg": cell.bg.map(|c| c.name()),
                    "char": cell.ch.to_string(),
                }));
            }
        }
        cells.push(row);
    }

    let json = serde_json::json!({
        "width": canvas.width,
        "height": canvas.height,
        "cells": cells,
        "non_empty_count": non_empty_count,
    });
    serde_json::to_string_pretty(&json).unwrap()
}

fn ansi_region(
    project: &crate::project::Project,
    x1: usize, y1: usize, x2: usize, y2: usize,
    format: crate::export::ColorFormat,
) -> String {
    // Create a sub-canvas from the region
    let canvas = &project.canvas;
    let mut sub = crate::canvas::Canvas::new_with_size(
        (x2 - x1 + 1).max(8),
        (y2 - y1 + 1).max(8),
    );
    for y in y1..=y2.min(canvas.height.saturating_sub(1)) {
        for x in x1..=x2.min(canvas.width.saturating_sub(1)) {
            if let Some(cell) = canvas.get(x, y) {
                sub.set(x - x1, y - y1, cell);
            }
        }
    }
    export::to_ansi(&sub, format)
}

fn plain_region(
    project: &crate::project::Project,
    x1: usize, y1: usize, x2: usize, y2: usize,
) -> String {
    let canvas = &project.canvas;
    let mut sub = crate::canvas::Canvas::new_with_size(
        (x2 - x1 + 1).max(8),
        (y2 - y1 + 1).max(8),
    );
    for y in y1..=y2.min(canvas.height.saturating_sub(1)) {
        for x in x1..=x2.min(canvas.width.saturating_sub(1)) {
            if let Some(cell) = canvas.get(x, y) {
                sub.set(x - x1, y - y1, cell);
            }
        }
    }
    export::to_plain_text(&sub)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_format_png() {
        assert_eq!(detect_format("out.png", &PreviewFormat::Auto), PreviewFormat::Png);
    }

    #[test]
    fn test_detect_format_txt() {
        assert_eq!(detect_format("out.txt", &PreviewFormat::Auto), PreviewFormat::Plain);
    }

    #[test]
    fn test_detect_format_json() {
        assert_eq!(detect_format("out.json", &PreviewFormat::Auto), PreviewFormat::Json);
    }

    #[test]
    fn test_detect_format_fallback_ansi() {
        assert_eq!(detect_format("out.ans", &PreviewFormat::Auto), PreviewFormat::Ansi);
        assert_eq!(detect_format("out", &PreviewFormat::Auto), PreviewFormat::Ansi);
    }

    #[test]
    fn test_detect_format_explicit_overrides() {
        assert_eq!(detect_format("out.png", &PreviewFormat::Plain), PreviewFormat::Plain);
        assert_eq!(detect_format("out.txt", &PreviewFormat::Json), PreviewFormat::Json);
    }

    #[test]
    fn test_parse_cell_size_valid() {
        assert_eq!(parse_cell_size("8x16"), Ok((8, 16)));
        assert_eq!(parse_cell_size("4x8"), Ok((4, 8)));
        assert_eq!(parse_cell_size("64x64"), Ok((64, 64)));
        assert_eq!(parse_cell_size("1x1"), Ok((1, 1)));
    }

    #[test]
    fn test_parse_cell_size_invalid() {
        assert!(parse_cell_size("0x16").is_err());
        assert!(parse_cell_size("8x0").is_err());
        assert!(parse_cell_size("65x16").is_err());
        assert!(parse_cell_size("abc").is_err());
        assert!(parse_cell_size("8").is_err());
        assert!(parse_cell_size("8x").is_err());
    }
}
