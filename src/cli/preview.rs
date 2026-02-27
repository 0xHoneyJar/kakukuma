use std::io;

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
        PreviewFormat::Ansi => {
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
    }
}

pub fn export_to_file(
    file: &str,
    output: &str,
    format: &PreviewFormat,
    color_format: &CliColorFormat,
) -> io::Result<()> {
    let project = load_project(file);
    let cf = to_color_format(color_format);

    let content = match format {
        PreviewFormat::Ansi => export::to_ansi(&project.canvas, cf),
        PreviewFormat::Plain => export::to_plain_text(&project.canvas),
        PreviewFormat::Json => json_preview(&project, None),
    };

    std::fs::write(output, &content)?;

    let format_str = match format {
        PreviewFormat::Ansi => "ansi",
        PreviewFormat::Plain => "plain",
        PreviewFormat::Json => "json",
    };
    let cf_str = match color_format {
        CliColorFormat::Truecolor => "truecolor",
        CliColorFormat::Color256 => "256",
        CliColorFormat::Color16 => "16",
    };

    let json = serde_json::json!({
        "exported": output,
        "format": format_str,
        "color_format": cf_str,
    });
    println!("{}", serde_json::to_string(&json).unwrap());
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
