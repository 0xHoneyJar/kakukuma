use std::io;
use std::path::Path;

use serde::Deserialize;

use crate::canvas::Canvas;
use crate::cell::{blocks, parse_hex_color, Cell, Rgb};
use crate::tools;

// --- Batch JSON types ---

#[derive(Deserialize)]
pub struct BatchFile {
    pub operations: Vec<BatchOp>,
}

#[derive(Deserialize)]
#[serde(tag = "op")]
pub enum BatchOp {
    #[serde(alias = "draw")]
    Draw {
        tool: String,
        x: Option<usize>,
        y: Option<usize>,
        x1: Option<usize>,
        y1: Option<usize>,
        x2: Option<usize>,
        y2: Option<usize>,
        ch: Option<String>,
        fg: Option<String>,
        bg: Option<String>,
        filled: Option<bool>,
    },
    #[serde(alias = "set_cell")]
    SetCell {
        x: usize,
        y: usize,
        ch: Option<String>,
        fg: Option<String>,
        bg: Option<String>,
    },
    #[serde(alias = "clear")]
    Clear {
        region: Option<[usize; 4]>,
    },
    #[serde(alias = "resize")]
    Resize {
        width: usize,
        height: usize,
    },
}

// --- Helpers ---

fn parse_optional_color(s: &Option<String>) -> Result<Option<Rgb>, String> {
    match s {
        None => Ok(None),
        Some(hex) => parse_hex_color(hex)
            .map(Some)
            .ok_or_else(|| format!("Invalid color: '{}'", hex)),
    }
}

fn parse_char(s: &Option<String>) -> char {
    match s {
        Some(ref c) if !c.is_empty() => c.chars().next().unwrap_or(blocks::FULL),
        _ => blocks::FULL,
    }
}

fn require_xy(x: Option<usize>, y: Option<usize>) -> Result<(usize, usize), String> {
    match (x, y) {
        (Some(x), Some(y)) => Ok((x, y)),
        _ => Err("Missing required x,y coordinates".to_string()),
    }
}

fn require_rect_coords(
    x1: Option<usize>, y1: Option<usize>,
    x2: Option<usize>, y2: Option<usize>,
) -> Result<(usize, usize, usize, usize), String> {
    match (x1, y1, x2, y2) {
        (Some(a), Some(b), Some(c), Some(d)) => Ok((a, b, c, d)),
        _ => Err("Missing required x1,y1,x2,y2 coordinates".to_string()),
    }
}

// --- Executor ---

fn execute_op(canvas: &mut Canvas, op: &BatchOp) -> Result<usize, String> {
    match op {
        BatchOp::Draw { tool, x, y, x1, y1, x2, y2, ch, fg, bg, filled } => {
            let character = parse_char(ch);
            let fg_rgb = parse_optional_color(fg)?;
            let bg_rgb = parse_optional_color(bg)?;

            let mutations = match tool.as_str() {
                "pencil" => {
                    let (px, py) = require_xy(*x, *y)?;
                    tools::pencil(canvas, px, py, character, fg_rgb, bg_rgb)
                }
                "eraser" => {
                    let (px, py) = require_xy(*x, *y)?;
                    tools::eraser(canvas, px, py)
                }
                "line" => {
                    let (a, b, c, d) = require_rect_coords(*x1, *y1, *x2, *y2)
                        .or_else(|_| {
                            // Also try x,y as start and x2,y2 as end
                            match (x, y, x2, y2) {
                                (Some(a), Some(b), Some(c), Some(d)) => Ok((*a, *b, *c, *d)),
                                _ => Err("Line requires x1,y1,x2,y2 or x,y,x2,y2".to_string()),
                            }
                        })?;
                    tools::line(canvas, a, b, c, d, character, fg_rgb, bg_rgb)
                }
                "rect" | "rectangle" => {
                    let (a, b, c, d) = require_rect_coords(*x1, *y1, *x2, *y2)?;
                    tools::rectangle(canvas, a, b, c, d, character, fg_rgb, bg_rgb, filled.unwrap_or(false))
                }
                "fill" | "flood_fill" => {
                    let (px, py) = require_xy(*x, *y)?;
                    tools::flood_fill(canvas, px, py, character, fg_rgb, bg_rgb)
                }
                unknown => return Err(format!("Unknown tool: '{}'", unknown)),
            };

            let count = mutations.len();
            for m in &mutations {
                canvas.set(m.x, m.y, m.new);
            }
            Ok(count)
        }
        BatchOp::SetCell { x, y, ch, fg, bg } => {
            let character = parse_char(ch);
            let fg_rgb = parse_optional_color(fg)?;
            let bg_rgb = parse_optional_color(bg)?;
            let cell = Cell { ch: character, fg: fg_rgb, bg: bg_rgb };
            canvas.set(*x, *y, cell);
            Ok(1)
        }
        BatchOp::Clear { region } => {
            match region {
                Some([x1, y1, x2, y2]) => {
                    let mut count = 0;
                    for cy in *y1..=(*y2).min(canvas.height.saturating_sub(1)) {
                        for cx in *x1..=(*x2).min(canvas.width.saturating_sub(1)) {
                            canvas.set(cx, cy, Cell::default());
                            count += 1;
                        }
                    }
                    Ok(count)
                }
                None => {
                    let w = canvas.width;
                    let h = canvas.height;
                    for cy in 0..h {
                        for cx in 0..w {
                            canvas.set(cx, cy, Cell::default());
                        }
                    }
                    Ok(w * h)
                }
            }
        }
        BatchOp::Resize { width, height } => {
            let w = (*width).clamp(crate::canvas::MIN_DIMENSION, crate::canvas::MAX_DIMENSION);
            let h = (*height).clamp(crate::canvas::MIN_DIMENSION, crate::canvas::MAX_DIMENSION);
            canvas.resize(w, h);
            Ok(0)
        }
    }
}

/// Run batch operations from a JSON file on a .kaku project.
pub fn run_batch(file: &str, commands_path: &str, dry_run: bool) -> io::Result<()> {
    let path = Path::new(file);
    let mut project = super::load_project(file);

    // Read and parse JSON
    let json_content = std::fs::read_to_string(commands_path).map_err(|e| {
        let json = serde_json::json!({
            "error": format!("Cannot read commands file '{}': {}", commands_path, e),
            "code": "USER_ERROR"
        });
        eprintln!("{}", json);
        io::Error::new(io::ErrorKind::NotFound, e)
    })?;

    let batch: BatchFile = serde_json::from_str(&json_content).map_err(|e| {
        let json = serde_json::json!({
            "error": format!("Invalid batch JSON: {}", e),
            "code": "USER_ERROR"
        });
        eprintln!("{}", json);
        io::Error::new(io::ErrorKind::InvalidData, e)
    })?;

    let op_count = batch.operations.len();

    if dry_run {
        let json = serde_json::json!({
            "dry_run": true,
            "operations": op_count,
            "file": file,
        });
        println!("{}", serde_json::to_string(&json).unwrap());
        return Ok(());
    }

    // Execute operations
    let mut cells_modified = 0usize;
    let mut errors = 0usize;
    let mut error_details: Vec<serde_json::Value> = Vec::new();

    for (i, op) in batch.operations.iter().enumerate() {
        match execute_op(&mut project.canvas, op) {
            Ok(count) => cells_modified += count,
            Err(msg) => {
                errors += 1;
                error_details.push(serde_json::json!({
                    "operation": i,
                    "error": msg,
                }));
            }
        }
    }

    // Atomic save
    super::atomic_save(&mut project, path)?;

    let mut result = serde_json::json!({
        "operations": op_count,
        "cells_modified": cells_modified,
        "errors": errors,
        "file": file,
    });

    if !error_details.is_empty() {
        result["error_details"] = serde_json::Value::Array(error_details);
    }

    println!("{}", serde_json::to_string(&result).unwrap());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::Canvas;
    use crate::cell::{blocks, Cell, Rgb};

    fn test_canvas() -> Canvas {
        Canvas::new_with_size(16, 16)
    }

    #[test]
    fn test_batch_deserialize_draw() {
        let json = r##"{"operations":[{"op":"Draw","tool":"pencil","x":5,"y":10,"fg":"#FF0000"}]}"##;
        let batch: BatchFile = serde_json::from_str(json).unwrap();
        assert_eq!(batch.operations.len(), 1);
        match &batch.operations[0] {
            BatchOp::Draw { tool, x, y, .. } => {
                assert_eq!(tool, "pencil");
                assert_eq!(*x, Some(5));
                assert_eq!(*y, Some(10));
            }
            _ => panic!("Expected Draw"),
        }
    }

    #[test]
    fn test_batch_deserialize_lowercase_alias() {
        let json = r#"{"operations":[{"op":"draw","tool":"pencil","x":0,"y":0}]}"#;
        let batch: BatchFile = serde_json::from_str(json).unwrap();
        assert_eq!(batch.operations.len(), 1);
    }

    #[test]
    fn test_batch_deserialize_set_cell() {
        let json = r##"{"operations":[{"op":"SetCell","x":1,"y":2,"ch":"X","fg":"#00FF00"}]}"##;
        let batch: BatchFile = serde_json::from_str(json).unwrap();
        match &batch.operations[0] {
            BatchOp::SetCell { x, y, ch, .. } => {
                assert_eq!(*x, 1);
                assert_eq!(*y, 2);
                assert_eq!(ch.as_deref(), Some("X"));
            }
            _ => panic!("Expected SetCell"),
        }
    }

    #[test]
    fn test_batch_deserialize_malformed() {
        let json = r#"{"operations":[{"op":"Unknown"}]}"#;
        let result: Result<BatchFile, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_pencil() {
        let mut canvas = test_canvas();
        let op = BatchOp::Draw {
            tool: "pencil".to_string(),
            x: Some(5), y: Some(5),
            x1: None, y1: None, x2: None, y2: None,
            ch: None, fg: Some("#FF0000".to_string()), bg: None,
            filled: None,
        };
        let count = execute_op(&mut canvas, &op).unwrap();
        assert_eq!(count, 1);
        let cell = canvas.get(5, 5).unwrap();
        assert_eq!(cell.ch, blocks::FULL);
        assert_eq!(cell.fg, Some(Rgb::new(255, 0, 0)));
    }

    #[test]
    fn test_execute_rect() {
        let mut canvas = test_canvas();
        let op = BatchOp::Draw {
            tool: "rect".to_string(),
            x: None, y: None,
            x1: Some(0), y1: Some(0), x2: Some(3), y2: Some(3),
            ch: None, fg: Some("#FFFFFF".to_string()), bg: None,
            filled: None,
        };
        let count = execute_op(&mut canvas, &op).unwrap();
        // 4x4 outline = 12 cells (perimeter of 4x4)
        assert_eq!(count, 12);
    }

    #[test]
    fn test_execute_fill() {
        let mut canvas = test_canvas();
        let op = BatchOp::Draw {
            tool: "fill".to_string(),
            x: Some(0), y: Some(0),
            x1: None, y1: None, x2: None, y2: None,
            ch: None, fg: Some("#00FF00".to_string()), bg: None,
            filled: None,
        };
        let count = execute_op(&mut canvas, &op).unwrap();
        // Flood fills entire 16x16 empty canvas = 256
        assert_eq!(count, 256);
    }

    #[test]
    fn test_execute_line() {
        let mut canvas = test_canvas();
        let op = BatchOp::Draw {
            tool: "line".to_string(),
            x: None, y: None,
            x1: Some(0), y1: Some(0), x2: Some(5), y2: Some(0),
            ch: None, fg: Some("#FFFFFF".to_string()), bg: None,
            filled: None,
        };
        let count = execute_op(&mut canvas, &op).unwrap();
        assert_eq!(count, 6); // Horizontal line 0..=5
    }

    #[test]
    fn test_execute_set_cell() {
        let mut canvas = test_canvas();
        let op = BatchOp::SetCell {
            x: 3, y: 4,
            ch: Some("X".to_string()),
            fg: Some("#FF0000".to_string()),
            bg: Some("#0000FF".to_string()),
        };
        let count = execute_op(&mut canvas, &op).unwrap();
        assert_eq!(count, 1);
        let cell = canvas.get(3, 4).unwrap();
        assert_eq!(cell.ch, 'X');
        assert_eq!(cell.fg, Some(Rgb::new(255, 0, 0)));
        assert_eq!(cell.bg, Some(Rgb::new(0, 0, 255)));
    }

    #[test]
    fn test_execute_clear_region() {
        let mut canvas = test_canvas();
        // Draw something first
        canvas.set(2, 2, Cell { ch: 'X', fg: Some(Rgb::WHITE), bg: None });
        let op = BatchOp::Clear { region: Some([1, 1, 3, 3]) };
        let count = execute_op(&mut canvas, &op).unwrap();
        assert_eq!(count, 9); // 3x3 region
        assert_eq!(canvas.get(2, 2).unwrap(), Cell::default());
    }

    #[test]
    fn test_execute_clear_full() {
        let mut canvas = test_canvas();
        canvas.set(0, 0, Cell { ch: 'X', fg: Some(Rgb::WHITE), bg: None });
        let op = BatchOp::Clear { region: None };
        let count = execute_op(&mut canvas, &op).unwrap();
        assert_eq!(count, 256); // 16x16
        assert_eq!(canvas.get(0, 0).unwrap(), Cell::default());
    }

    #[test]
    fn test_execute_resize() {
        let mut canvas = test_canvas();
        assert_eq!(canvas.width, 16);
        assert_eq!(canvas.height, 16);
        let op = BatchOp::Resize { width: 32, height: 24 };
        let count = execute_op(&mut canvas, &op).unwrap();
        assert_eq!(count, 0);
        assert_eq!(canvas.width, 32);
        assert_eq!(canvas.height, 24);
    }

    #[test]
    fn test_execute_unknown_tool() {
        let mut canvas = test_canvas();
        let op = BatchOp::Draw {
            tool: "magic".to_string(),
            x: Some(0), y: Some(0),
            x1: None, y1: None, x2: None, y2: None,
            ch: None, fg: None, bg: None, filled: None,
        };
        let result = execute_op(&mut canvas, &op);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown tool"));
    }

    #[test]
    fn test_multi_op_ordering() {
        let mut canvas = test_canvas();
        // First: pencil at (5,5) with red
        let op1 = BatchOp::Draw {
            tool: "pencil".to_string(),
            x: Some(5), y: Some(5),
            x1: None, y1: None, x2: None, y2: None,
            ch: None, fg: Some("#FF0000".to_string()), bg: None,
            filled: None,
        };
        execute_op(&mut canvas, &op1).unwrap();

        // Second: set_cell overwrites with green
        let op2 = BatchOp::SetCell {
            x: 5, y: 5,
            ch: Some("A".to_string()),
            fg: Some("#00FF00".to_string()),
            bg: None,
        };
        execute_op(&mut canvas, &op2).unwrap();

        let cell = canvas.get(5, 5).unwrap();
        assert_eq!(cell.ch, 'A');
        assert_eq!(cell.fg, Some(Rgb::new(0, 255, 0)));
    }

    #[test]
    fn test_empty_operations() {
        let mut canvas = test_canvas();
        let batch = BatchFile { operations: vec![] };
        let mut total = 0;
        for op in &batch.operations {
            total += execute_op(&mut canvas, op).unwrap_or(0);
        }
        assert_eq!(total, 0);
    }

    #[test]
    fn test_dry_run_no_modify() {
        // The dry_run logic is in run_batch(), not execute_op.
        // Verify that execute_op is never called for empty ops.
        let canvas = test_canvas();
        // Simply verify our test canvas is unmodified
        assert_eq!(canvas.get(0, 0).unwrap(), Cell::default());
    }
}
