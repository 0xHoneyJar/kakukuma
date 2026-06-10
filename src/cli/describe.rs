//! Semantic canvas perception for agents: `describe` and `snapshot`.
//!
//! `describe` emits a compact, token-efficient representation of the canvas:
//! a legend mapping each distinct cell appearance (char + fg + bg) to a
//! single symbol, plus one grid row per canvas row. A 48x32 canvas costs
//! ~1.6k characters instead of ~16k cells of JSON.
//!
//! `snapshot` is the one-call perception loop primitive: it writes a PNG
//! (for vision) and prints the describe output (for structured reasoning).

use std::collections::HashMap;
use std::io;
use std::path::Path;

use crate::canvas::Canvas;
use crate::cell::Cell;
use crate::cli::{cli_error, load_project};
use crate::export;

/// Symbols assigned to distinct cell appearances, in order of frequency.
/// '.' is reserved for empty cells, '?' for overflow beyond 62 appearances.
const SYMBOLS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

pub struct LegendEntry {
    pub symbol: char,
    pub cell: Cell,
    pub count: usize,
}

/// Hashable cell appearance: (char, fg, bg) with colors as raw tuples.
type Appearance = (char, Option<(u8, u8, u8)>, Option<(u8, u8, u8)>);

fn appearance_of(cell: &Cell) -> Appearance {
    (
        cell.ch,
        cell.fg.map(|c| (c.r, c.g, c.b)),
        cell.bg.map(|c| (c.r, c.g, c.b)),
    )
}

/// Build the legend (distinct appearances by frequency) and the symbol grid.
pub fn indexed_grid(canvas: &Canvas) -> (Vec<LegendEntry>, Vec<String>, usize) {
    // Count distinct non-empty appearances
    let mut counts: HashMap<Appearance, usize> = HashMap::new();
    for y in 0..canvas.height {
        for x in 0..canvas.width {
            if let Some(cell) = canvas.get(x, y) {
                if !cell.is_empty() {
                    *counts.entry(appearance_of(&cell)).or_insert(0) += 1;
                }
            }
        }
    }

    // Sort by frequency descending, then by key for determinism
    let mut sorted: Vec<_> = counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let overflow = sorted.len().saturating_sub(SYMBOLS.len());
    let mut symbol_of: HashMap<Appearance, char> = HashMap::new();
    let mut legend = Vec::new();
    for (i, (key, count)) in sorted.into_iter().enumerate() {
        let symbol = if i < SYMBOLS.len() {
            SYMBOLS[i] as char
        } else {
            '?'
        };
        symbol_of.insert(key, symbol);
        if symbol != '?' {
            let cell = Cell {
                ch: key.0,
                fg: key.1.map(|(r, g, b)| crate::cell::Rgb { r, g, b }),
                bg: key.2.map(|(r, g, b)| crate::cell::Rgb { r, g, b }),
            };
            legend.push(LegendEntry { symbol, cell, count });
        }
    }

    // Render the grid
    let mut rows = Vec::with_capacity(canvas.height);
    for y in 0..canvas.height {
        let mut row = String::with_capacity(canvas.width);
        for x in 0..canvas.width {
            let cell = canvas.get(x, y).unwrap_or_default();
            if cell.is_empty() {
                row.push('.');
            } else {
                row.push(*symbol_of.get(&appearance_of(&cell)).unwrap_or(&'?'));
            }
        }
        rows.push(row);
    }

    (legend, rows, overflow)
}

fn legend_json(legend: &[LegendEntry]) -> Vec<serde_json::Value> {
    legend
        .iter()
        .map(|e| {
            serde_json::json!({
                "symbol": e.symbol.to_string(),
                "char": e.cell.ch.to_string(),
                "fg": e.cell.fg.map(|c| c.name()),
                "bg": e.cell.bg.map(|c| c.name()),
                "count": e.count,
            })
        })
        .collect()
}

/// Shared describe payload used by both `describe` and `snapshot`.
pub fn describe_json(canvas: &Canvas) -> serde_json::Value {
    let (legend, rows, overflow) = indexed_grid(canvas);

    let total = canvas.width * canvas.height;
    let filled: usize = legend.iter().map(|e| e.count).sum();
    let fill_pct = if total > 0 {
        (filled as f64 / total as f64 * 10000.0).round() / 100.0
    } else {
        0.0
    };

    let bbox = export::bounding_box(canvas).map(|(x1, y1, x2, y2)| {
        serde_json::json!({"min_x": x1, "min_y": y1, "max_x": x2, "max_y": y2})
    });

    let mut json = serde_json::json!({
        "width": canvas.width,
        "height": canvas.height,
        "hash": canvas.content_hash(),
        "filled": filled,
        "fill_percent": fill_pct,
        "bounding_box": bbox,
        "legend": legend_json(&legend),
        "grid": rows,
    });
    if overflow > 0 {
        json["legend_overflow"] = serde_json::json!({
            "appearances_beyond_62": overflow,
            "note": "cells shown as '?' in grid; use 'kakukuma inspect' for exact values",
        });
    }
    json
}

/// `kakukuma describe <file> [--plain]`
pub fn run(file: &str, plain: bool) -> io::Result<()> {
    let project = load_project(file);
    let canvas = &project.canvas;

    if plain {
        let (legend, rows, overflow) = indexed_grid(canvas);
        println!(
            "{}x{}  hash:{}  filled:{}",
            canvas.width,
            canvas.height,
            canvas.content_hash(),
            legend.iter().map(|e| e.count).sum::<usize>(),
        );
        println!("legend:");
        for e in &legend {
            println!(
                "  {} = '{}' fg:{} bg:{} ({} cells)",
                e.symbol,
                e.cell.ch,
                e.cell.fg.map(|c| c.name()).unwrap_or_else(|| "-".into()),
                e.cell.bg.map(|c| c.name()).unwrap_or_else(|| "-".into()),
                e.count,
            );
        }
        if overflow > 0 {
            println!("  ? = {} more appearances (overflow)", overflow);
        }
        println!("grid:");
        for row in rows {
            println!("  {}", row);
        }
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&describe_json(canvas)).unwrap()
        );
    }
    Ok(())
}

/// `kakukuma snapshot <file> [PNG] [--cell-size WxH] [--scale N]`
///
/// Writes a PNG render and prints the describe payload (with the PNG path)
/// in one call — the agent perception loop primitive.
pub fn snapshot(
    file: &str,
    png_path: Option<&str>,
    cell_size: &str,
    scale: u32,
) -> io::Result<()> {
    let project = load_project(file);
    let canvas = &project.canvas;

    let out = match png_path {
        Some(p) => p.to_string(),
        None => {
            // Default: next to the .kaku file
            let p = Path::new(file);
            p.with_extension("png").to_string_lossy().into_owned()
        }
    };

    let (cw, ch) = parse_cell_size(cell_size);
    let img = export::to_png(canvas, cw, ch, scale, false);
    img.save(&out).unwrap_or_else(|e| {
        cli_error(&format!("Failed to write PNG '{}': {}", out, e));
    });

    let mut json = describe_json(canvas);
    json["png"] = serde_json::json!(out);
    json["png_pixels"] = serde_json::json!({
        "width": img.width(),
        "height": img.height(),
    });
    println!("{}", serde_json::to_string_pretty(&json).unwrap());
    Ok(())
}

fn parse_cell_size(s: &str) -> (u32, u32) {
    let parts: Vec<&str> = s.split('x').collect();
    if parts.len() == 2 {
        if let (Ok(w), Ok(h)) = (parts[0].trim().parse(), parts[1].trim().parse()) {
            if w >= 1 && h >= 1 && w <= 64 && h <= 64 {
                return (w, h);
            }
        }
    }
    cli_error(&format!(
        "Invalid cell size '{}'. Expected WxH (e.g. 8x16)",
        s
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::{blocks, Rgb};

    fn canvas_with_art() -> Canvas {
        let mut c = Canvas::new_with_size(8, 8);
        let red = Cell {
            ch: blocks::FULL,
            fg: Some(Rgb::new(255, 0, 0)),
            bg: None,
        };
        let blue = Cell {
            ch: blocks::FULL,
            fg: Some(Rgb::new(0, 0, 255)),
            bg: None,
        };
        // 3 red cells, 1 blue cell
        c.set(0, 0, red);
        c.set(1, 0, red);
        c.set(2, 0, red);
        c.set(0, 1, blue);
        c
    }

    #[test]
    fn test_indexed_grid_legend_by_frequency() {
        let canvas = canvas_with_art();
        let (legend, rows, overflow) = indexed_grid(&canvas);
        assert_eq!(overflow, 0);
        assert_eq!(legend.len(), 2);
        // Most frequent appearance gets 'a'
        assert_eq!(legend[0].symbol, 'a');
        assert_eq!(legend[0].count, 3);
        assert_eq!(legend[0].cell.fg, Some(Rgb::new(255, 0, 0)));
        assert_eq!(legend[1].symbol, 'b');
        assert_eq!(legend[1].count, 1);
        // Grid encodes positions
        assert_eq!(rows[0], "aaa.....");
        assert_eq!(rows[1], "b.......");
        assert_eq!(rows[7], "........");
    }

    #[test]
    fn test_indexed_grid_empty_canvas() {
        let canvas = Canvas::new_with_size(8, 8);
        let (legend, rows, overflow) = indexed_grid(&canvas);
        assert!(legend.is_empty());
        assert_eq!(overflow, 0);
        assert!(rows.iter().all(|r| r == "........"));
    }

    #[test]
    fn test_describe_json_shape() {
        let canvas = canvas_with_art();
        let json = describe_json(&canvas);
        assert_eq!(json["width"], 8);
        assert_eq!(json["height"], 8);
        assert_eq!(json["filled"], 4);
        assert_eq!(json["hash"], canvas.content_hash());
        assert_eq!(json["grid"].as_array().unwrap().len(), 8);
        assert_eq!(json["legend"].as_array().unwrap().len(), 2);
        let bbox = &json["bounding_box"];
        assert_eq!(bbox["min_x"], 0);
        assert_eq!(bbox["max_x"], 2);
        assert_eq!(bbox["max_y"], 1);
    }

    #[test]
    fn test_grid_is_deterministic() {
        let canvas = canvas_with_art();
        let (_, rows1, _) = indexed_grid(&canvas);
        let (_, rows2, _) = indexed_grid(&canvas);
        assert_eq!(rows1, rows2);
    }

    #[test]
    fn test_parse_cell_size_valid() {
        assert_eq!(parse_cell_size("8x16"), (8, 16));
        assert_eq!(parse_cell_size("1x1"), (1, 1));
    }
}
