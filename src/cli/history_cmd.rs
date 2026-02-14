use std::io;
use std::path::Path;

use crate::cli::{atomic_save, load_project};
use crate::oplog;

pub fn undo(file: &str, count: usize) -> io::Result<()> {
    let log_path = oplog::log_path(Path::new(file));
    let undone = oplog::pop_for_undo(&log_path, count)?;

    // Apply inverse mutations to the canvas
    let path = Path::new(file);
    let mut project = load_project(file);

    let mut cells_restored = 0usize;
    for entry in &undone {
        for m in &entry.mutations {
            let old_cell = m.old.to_cell();
            project.canvas.set(m.x, m.y, old_cell);
            cells_restored += 1;
        }
    }

    atomic_save(&mut project, path)?;

    let json = serde_json::json!({
        "ok": true,
        "undone": undone.len(),
        "cells_restored": cells_restored,
    });
    println!("{}", serde_json::to_string(&json).unwrap());
    Ok(())
}

pub fn redo(file: &str, count: usize) -> io::Result<()> {
    let log_path = oplog::log_path(Path::new(file));
    let redone = oplog::push_for_redo(&log_path, count)?;

    // Re-apply forward mutations to the canvas
    let path = Path::new(file);
    let mut project = load_project(file);

    let mut cells_applied = 0usize;
    for entry in &redone {
        for m in &entry.mutations {
            let new_cell = m.new.to_cell();
            project.canvas.set(m.x, m.y, new_cell);
            cells_applied += 1;
        }
    }

    atomic_save(&mut project, path)?;

    let json = serde_json::json!({
        "ok": true,
        "redone": redone.len(),
        "cells_applied": cells_applied,
    });
    println!("{}", serde_json::to_string(&json).unwrap());
    Ok(())
}

pub fn history(file: &str, full: bool) -> io::Result<()> {
    let log_path = oplog::log_path(Path::new(file));
    let (header, entries) = oplog::read_log(&log_path)?;

    if entries.is_empty() {
        let json = serde_json::json!({
            "pointer": 0,
            "total": 0,
            "entries": [],
            "message": "No operations recorded",
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
        return Ok(());
    }

    let entries_json: Vec<_> = entries.iter().enumerate().map(|(i, e)| {
        let active = i < header.pointer;
        if full {
            let mutations: Vec<_> = e.mutations.iter().map(|m| {
                serde_json::json!({
                    "x": m.x,
                    "y": m.y,
                    "old": {"ch": m.old.ch.to_string(), "fg": m.old.fg, "bg": m.old.bg},
                    "new": {"ch": m.new.ch.to_string(), "fg": m.new.fg, "bg": m.new.bg},
                })
            }).collect();
            serde_json::json!({
                "index": i,
                "active": active,
                "timestamp": e.timestamp,
                "command": e.command,
                "mutation_count": e.mutations.len(),
                "mutations": mutations,
            })
        } else {
            serde_json::json!({
                "index": i,
                "active": active,
                "timestamp": e.timestamp,
                "command": e.command,
                "mutation_count": e.mutations.len(),
            })
        }
    }).collect();

    let json = serde_json::json!({
        "pointer": header.pointer,
        "total": header.total,
        "entries": entries_json,
    });
    println!("{}", serde_json::to_string_pretty(&json).unwrap());
    Ok(())
}
