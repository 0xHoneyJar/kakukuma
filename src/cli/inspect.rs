use std::io;

use crate::cli::load_project;

pub fn run(
    file: &str,
    coord: Option<(usize, usize)>,
    region: Option<(usize, usize, usize, usize)>,
    row: Option<usize>,
    col: Option<usize>,
) -> io::Result<()> {
    let project = load_project(file);
    let canvas = &project.canvas;

    if let Some((x, y)) = coord {
        // Single cell inspection
        if x >= canvas.width || y >= canvas.height {
            eprintln!("Error: Position ({}, {}) exceeds canvas dimensions ({}x{})", x, y, canvas.width, canvas.height);
            std::process::exit(1);
        }
        let cell = canvas.get(x, y).unwrap();
        let json = serde_json::json!({
            "x": x,
            "y": y,
            "fg": cell.fg.map(|c| c.name()),
            "bg": cell.bg.map(|c| c.name()),
            "char": cell.ch.to_string(),
            "empty": cell.is_empty(),
        });
        println!("{}", serde_json::to_string(&json).unwrap());
    } else if let Some((x1, y1, x2, y2)) = region {
        // Region inspection — non-empty cells only
        let mut cells = Vec::new();
        let x2 = x2.min(canvas.width.saturating_sub(1));
        let y2 = y2.min(canvas.height.saturating_sub(1));
        for y in y1..=y2 {
            for x in x1..=x2 {
                if let Some(cell) = canvas.get(x, y) {
                    if !cell.is_empty() {
                        cells.push(serde_json::json!({
                            "x": x,
                            "y": y,
                            "fg": cell.fg.map(|c| c.name()),
                            "bg": cell.bg.map(|c| c.name()),
                            "char": cell.ch.to_string(),
                            "empty": false,
                        }));
                    }
                }
            }
        }
        println!("{}", serde_json::to_string(&cells).unwrap());
    } else if let Some(r) = row {
        // Row inspection
        if r >= canvas.height {
            eprintln!("Error: Row {} exceeds canvas height ({})", r, canvas.height);
            std::process::exit(1);
        }
        let mut cells = Vec::new();
        for x in 0..canvas.width {
            if let Some(cell) = canvas.get(x, r) {
                cells.push(serde_json::json!({
                    "x": x,
                    "y": r,
                    "fg": cell.fg.map(|c| c.name()),
                    "bg": cell.bg.map(|c| c.name()),
                    "char": cell.ch.to_string(),
                    "empty": cell.is_empty(),
                }));
            }
        }
        println!("{}", serde_json::to_string(&cells).unwrap());
    } else if let Some(c) = col {
        // Column inspection
        if c >= canvas.width {
            eprintln!("Error: Column {} exceeds canvas width ({})", c, canvas.width);
            std::process::exit(1);
        }
        let mut cells = Vec::new();
        for y in 0..canvas.height {
            if let Some(cell) = canvas.get(c, y) {
                cells.push(serde_json::json!({
                    "x": c,
                    "y": y,
                    "fg": cell.fg.map(|c| c.name()),
                    "bg": cell.bg.map(|c| c.name()),
                    "char": cell.ch.to_string(),
                    "empty": cell.is_empty(),
                }));
            }
        }
        println!("{}", serde_json::to_string(&cells).unwrap());
    } else {
        eprintln!("Error: Specify a coordinate, --region, --row, or --col");
        std::process::exit(1);
    }

    Ok(())
}
