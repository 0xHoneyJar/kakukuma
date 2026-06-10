use std::io;

use crate::cell::Cell;
use crate::cli::load_project;
use crate::oplog;
use crate::project::Project;

pub fn run(file1: &str, file2: Option<&str>, before: bool) -> io::Result<()> {
    if before {
        cmd_diff_before(file1)
    } else if let Some(f2) = file2 {
        cmd_diff_files(file1, f2)
    } else {
        eprintln!("Error: Specify a second file or use --before");
        std::process::exit(1);
    }
}

fn cmd_diff_files(file1: &str, file2: &str) -> io::Result<()> {
    let p1 = load_project(file1);
    let p2 = load_project(file2);
    let result = diff_canvases(&p1, &p2);
    println!("{}", serde_json::to_string_pretty(&result).unwrap());
    Ok(())
}

fn cmd_diff_before(file: &str) -> io::Result<()> {
    let project = load_project(file);
    let log_path = oplog::log_path(std::path::Path::new(file));
    let entries = oplog::active_entries(&log_path)?;

    if entries.is_empty() {
        eprintln!("Error: No operations recorded — cannot diff against previous state");
        std::process::exit(1);
    }

    let last = &entries[entries.len() - 1];

    let mut changes = Vec::new();
    let (mut added, mut removed, mut modified) = (0usize, 0usize, 0usize);

    for m in &last.mutations {
        let before_cell = m.old.to_cell();
        let after_cell = m.new.to_cell();
        let before_empty = before_cell.is_empty();
        let after_empty = after_cell.is_empty();
        match (before_empty, after_empty) {
            (true, false) => added += 1,
            (false, true) => removed += 1,
            _ => modified += 1,
        }
        changes.push(serde_json::json!({
            "x": m.x,
            "y": m.y,
            "before": cell_json(&before_cell),
            "after": cell_json(&after_cell),
        }));
    }

    let total_cells = project.canvas.width * project.canvas.height;
    let unchanged = total_cells - changes.len();

    let result = serde_json::json!({
        "changes": changes,
        "added": added,
        "removed": removed,
        "modified": modified,
        "unchanged": unchanged,
    });
    println!("{}", serde_json::to_string_pretty(&result).unwrap());
    Ok(())
}

fn diff_canvases(p1: &Project, p2: &Project) -> serde_json::Value {
    let c1 = &p1.canvas;
    let c2 = &p2.canvas;
    let w = c1.width.max(c2.width);
    let h = c1.height.max(c2.height);

    let mut changes = Vec::new();
    let (mut added, mut removed, mut modified, mut unchanged) = (0usize, 0usize, 0usize, 0usize);

    for y in 0..h {
        for x in 0..w {
            let a = c1.get(x, y).unwrap_or(Cell::default());
            let b = c2.get(x, y).unwrap_or(Cell::default());
            if a != b {
                let a_empty = a.is_empty();
                let b_empty = b.is_empty();
                match (a_empty, b_empty) {
                    (true, false) => added += 1,
                    (false, true) => removed += 1,
                    _ => modified += 1,
                }
                changes.push(serde_json::json!({
                    "x": x,
                    "y": y,
                    "before": cell_json(&a),
                    "after": cell_json(&b),
                }));
            } else {
                unchanged += 1;
            }
        }
    }

    serde_json::json!({
        "changes": changes,
        "added": added,
        "removed": removed,
        "modified": modified,
        "unchanged": unchanged,
    })
}

fn cell_json(cell: &Cell) -> serde_json::Value {
    serde_json::json!({
        "fg": cell.fg.map(|c| c.name()),
        "bg": cell.bg.map(|c| c.name()),
        "char": cell.ch.to_string(),
    })
}
