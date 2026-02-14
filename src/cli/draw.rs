use std::io;
use std::path::Path;

use crate::cell::blocks;
use crate::cli::{DrawOpts, DrawTool, atomic_save, cli_error, load_project, resolve_colors, to_symmetry_mode};
use crate::history::CellMutation;
use crate::oplog;
use crate::symmetry::apply_symmetry;
use crate::tools;

pub fn run(tool: DrawTool) -> io::Result<()> {
    match tool {
        DrawTool::Pencil { file, coord, opts } => cmd_pencil(&file, coord, &opts),
        DrawTool::Eraser { file, coord, region } => cmd_eraser(&file, coord, region),
        DrawTool::Line { file, from, to, opts } => cmd_line(&file, from, to, &opts),
        DrawTool::Rect { file, from, to, filled, opts } => cmd_rect(&file, from, to, filled, &opts),
        DrawTool::Fill { file, coord, opts } => cmd_fill(&file, coord, &opts),
        DrawTool::Eyedropper { file, coord } => cmd_eyedropper(&file, coord),
    }
}

fn apply_and_save(
    file: &str,
    tool_name: &str,
    mutations: Vec<CellMutation>,
    opts: Option<&DrawOpts>,
) -> io::Result<()> {
    let path = Path::new(file);
    let mut project = load_project(file);

    let sym_mode = opts.map(|o| to_symmetry_mode(&o.symmetry))
        .unwrap_or(crate::symmetry::SymmetryMode::Off);

    let mutations = apply_symmetry(
        mutations,
        sym_mode,
        project.canvas.width,
        project.canvas.height,
    );

    // Apply mutations to canvas
    for m in &mutations {
        project.canvas.set(m.x, m.y, m.new);
    }

    let cells_modified = mutations.len();

    // Log operation (unless --no-log)
    let no_log = opts.map(|o| o.no_log).unwrap_or(false);
    if !no_log && !mutations.is_empty() {
        let log_path = oplog::log_path(path);
        let entry = oplog::make_entry(tool_name, &mutations);
        oplog::append(&log_path, entry)?;
    }

    // Atomic save
    atomic_save(&mut project, path)?;

    let sym_label = opts
        .map(|o| format!("{:?}", o.symmetry).to_lowercase())
        .unwrap_or_else(|| "off".to_string());

    let json = serde_json::json!({
        "ok": true,
        "cells_modified": cells_modified,
        "tool": tool_name,
        "symmetry": sym_label,
    });
    println!("{}", serde_json::to_string(&json).unwrap());
    Ok(())
}

fn cmd_pencil(file: &str, coord: (usize, usize), opts: &DrawOpts) -> io::Result<()> {
    let project = load_project(file);
    let (fg, bg) = resolve_colors(opts);
    let ch = opts.ch.unwrap_or(blocks::FULL);

    let (x, y) = coord;
    validate_coords(x, y, &project.canvas);

    let mutations = tools::pencil(&project.canvas, x, y, ch, fg, bg);
    drop(project); // Release the loaded project before apply_and_save reloads

    apply_and_save(file, "pencil", mutations, Some(opts))
}

fn cmd_eraser(file: &str, coord: (usize, usize), region: Option<(usize, usize, usize, usize)>) -> io::Result<()> {
    let project = load_project(file);
    let (x, y) = coord;

    let mutations = if let Some((x1, y1, x2, y2)) = region {
        let mut all = Vec::new();
        for ry in y1..=y2 {
            for rx in x1..=x2 {
                all.extend(tools::eraser(&project.canvas, rx, ry));
            }
        }
        all
    } else {
        validate_coords(x, y, &project.canvas);
        tools::eraser(&project.canvas, x, y)
    };
    drop(project);

    apply_and_save(file, "eraser", mutations, None)
}

fn cmd_line(file: &str, from: (usize, usize), to: (usize, usize), opts: &DrawOpts) -> io::Result<()> {
    let project = load_project(file);
    let (fg, bg) = resolve_colors(opts);
    let ch = opts.ch.unwrap_or(blocks::FULL);

    let mutations = tools::line(&project.canvas, from.0, from.1, to.0, to.1, ch, fg, bg);
    drop(project);

    apply_and_save(file, "line", mutations, Some(opts))
}

fn cmd_rect(file: &str, from: (usize, usize), to: (usize, usize), filled: bool, opts: &DrawOpts) -> io::Result<()> {
    let project = load_project(file);
    let (fg, bg) = resolve_colors(opts);
    let ch = opts.ch.unwrap_or(blocks::FULL);

    let mutations = tools::rectangle(&project.canvas, from.0, from.1, to.0, to.1, ch, fg, bg, filled);
    drop(project);

    apply_and_save(file, "rect", mutations, Some(opts))
}

fn cmd_fill(file: &str, coord: (usize, usize), opts: &DrawOpts) -> io::Result<()> {
    let project = load_project(file);
    let (fg, bg) = resolve_colors(opts);
    let ch = opts.ch.unwrap_or(blocks::FULL);

    let (x, y) = coord;
    validate_coords(x, y, &project.canvas);

    let mutations = tools::flood_fill(&project.canvas, x, y, ch, fg, bg);
    drop(project);

    apply_and_save(file, "fill", mutations, Some(opts))
}

fn cmd_eyedropper(file: &str, coord: (usize, usize)) -> io::Result<()> {
    let project = load_project(file);
    let (x, y) = coord;
    validate_coords(x, y, &project.canvas);

    match tools::eyedropper(&project.canvas, x, y) {
        Some((fg, bg, ch)) => {
            let json = serde_json::json!({
                "x": x,
                "y": y,
                "fg": fg.map(|c| c.name()),
                "bg": bg.map(|c| c.name()),
                "char": ch.to_string(),
            });
            println!("{}", serde_json::to_string(&json).unwrap());
            Ok(())
        }
        None => {
            cli_error(&format!("Position ({}, {}) is out of bounds", x, y));
        }
    }
}

fn validate_coords(x: usize, y: usize, canvas: &crate::canvas::Canvas) {
    if x >= canvas.width || y >= canvas.height {
        cli_error(&format!(
            "Position ({}, {}) exceeds canvas dimensions ({}x{})",
            x, y, canvas.width, canvas.height
        ));
    }
}
