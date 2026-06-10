use std::collections::HashMap;
use std::io;

use crate::cli::load_project;

pub fn run(file: &str) -> io::Result<()> {
    let project = load_project(file);
    let canvas = &project.canvas;

    let total_cells = canvas.width * canvas.height;
    let mut non_empty = 0usize;
    let mut unique_chars: HashMap<char, usize> = HashMap::new();
    let mut fg_colors: HashMap<String, usize> = HashMap::new();
    let mut bg_colors: HashMap<String, usize> = HashMap::new();

    // Bounding box
    let mut min_x = canvas.width;
    let mut min_y = canvas.height;
    let mut max_x = 0usize;
    let mut max_y = 0usize;

    for y in 0..canvas.height {
        for x in 0..canvas.width {
            if let Some(cell) = canvas.get(x, y) {
                if !cell.is_empty() {
                    non_empty += 1;
                    *unique_chars.entry(cell.ch).or_insert(0) += 1;
                    if let Some(fg) = cell.fg {
                        *fg_colors.entry(fg.name()).or_insert(0) += 1;
                    }
                    if let Some(bg) = cell.bg {
                        *bg_colors.entry(bg.name()).or_insert(0) += 1;
                    }
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                }
            }
        }
    }

    let empty = total_cells - non_empty;
    let fill_pct = if total_cells > 0 {
        (non_empty as f64 / total_cells as f64) * 100.0
    } else {
        0.0
    };

    // Bounding box (null if empty)
    let bounding_box = if non_empty > 0 {
        serde_json::json!({"min_x": min_x, "min_y": min_y, "max_x": max_x, "max_y": max_y})
    } else {
        serde_json::Value::Null
    };

    // Symmetry scores
    let (h_score, v_score) = compute_symmetry_scores(canvas);

    // FG color distribution sorted by count descending
    let mut fg_sorted: Vec<_> = fg_colors.into_iter().collect();
    fg_sorted.sort_by(|a, b| b.1.cmp(&a.1));
    let fg_dist: Vec<_> = fg_sorted.iter()
        .map(|(c, n)| {
            let pct = if non_empty > 0 { (*n as f64 / non_empty as f64) * 100.0 } else { 0.0 };
            serde_json::json!({"color": c, "count": n, "percent": round2(pct)})
        })
        .collect();

    // BG color distribution sorted by count descending
    let mut bg_sorted: Vec<_> = bg_colors.into_iter().collect();
    bg_sorted.sort_by(|a, b| b.1.cmp(&a.1));
    let bg_dist: Vec<_> = bg_sorted.iter()
        .map(|(c, n)| {
            let pct = if non_empty > 0 { (*n as f64 / non_empty as f64) * 100.0 } else { 0.0 };
            serde_json::json!({"color": c, "count": n, "percent": round2(pct)})
        })
        .collect();

    // Character distribution sorted by count descending
    let mut char_sorted: Vec<_> = unique_chars.into_iter().collect();
    char_sorted.sort_by(|a, b| b.1.cmp(&a.1));
    let char_dist: Vec<_> = char_sorted.iter()
        .map(|(ch, n)| {
            let pct = if non_empty > 0 { (*n as f64 / non_empty as f64) * 100.0 } else { 0.0 };
            serde_json::json!({"char": ch.to_string(), "count": n, "percent": round2(pct)})
        })
        .collect();

    let json = serde_json::json!({
        "canvas": {
            "width": canvas.width,
            "height": canvas.height,
            "total_cells": total_cells,
        },
        "fill": {
            "empty": empty,
            "filled": non_empty,
            "fill_percent": round2(fill_pct),
        },
        "colors": {
            "unique_fg": fg_sorted.len(),
            "unique_bg": bg_sorted.len(),
            "fg_distribution": fg_dist,
            "bg_distribution": bg_dist,
        },
        "characters": {
            "unique": char_sorted.len(),
            "distribution": char_dist,
        },
        "bounding_box": bounding_box,
        "symmetry_score": {
            "horizontal": round2(h_score),
            "vertical": round2(v_score),
        },
    });
    println!("{}", serde_json::to_string_pretty(&json).unwrap());
    Ok(())
}

/// Round to 2 decimal places.
fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// Compute horizontal and vertical symmetry scores (0.0-1.0).
/// Compares each cell with its mirror. Empty-empty pairs count as matching.
fn compute_symmetry_scores(canvas: &crate::canvas::Canvas) -> (f64, f64) {
    let w = canvas.width;
    let h = canvas.height;
    let total = w * h;
    if total == 0 {
        return (1.0, 1.0);
    }

    // Horizontal symmetry: mirror across vertical center axis (left-right)
    let mut h_matches = 0usize;
    for y in 0..h {
        for x in 0..w {
            let mirror_x = w - 1 - x;
            let a = canvas.get(x, y).unwrap_or_default();
            let b = canvas.get(mirror_x, y).unwrap_or_default();
            if a == b {
                h_matches += 1;
            }
        }
    }

    // Vertical symmetry: mirror across horizontal center axis (top-bottom)
    let mut v_matches = 0usize;
    for y in 0..h {
        for x in 0..w {
            let mirror_y = h - 1 - y;
            let a = canvas.get(x, y).unwrap_or_default();
            let b = canvas.get(x, mirror_y).unwrap_or_default();
            if a == b {
                v_matches += 1;
            }
        }
    }

    (h_matches as f64 / total as f64, v_matches as f64 / total as f64)
}
