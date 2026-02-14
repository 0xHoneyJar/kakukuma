# SDD: Kakukuma Agent Mode — Programmatic Drawing Interface

> **Version**: 1.0
> **Status**: Draft
> **Date**: 2026-02-14
> **PRD**: `grimoires/loa/prd.md`

## 1. Executive Summary

This document designs a CLI subcommand interface for Kakukuma that enables AI agents to create, manipulate, and inspect ANSI art programmatically. The design preserves all existing TUI behavior while adding stateless CLI commands that operate on `.kaku` project files.

The core insight from codebase analysis: the existing modules (`tools.rs`, `canvas.rs`, `cell.rs`, `symmetry.rs`, `export.rs`, `project.rs`) are already cleanly separated from the TUI. Tool functions return `Vec<CellMutation>` independently of any UI state. This means the CLI can directly compose these modules without a wrapper layer, keeping the implementation minimal.

## 2. System Architecture

### 2.1 High-Level Component Diagram

```
┌─────────────────────────────────────────────────────────┐
│                     main.rs                              │
│  ┌──────────────┐            ┌───────────────────────┐  │
│  │  No subcommand│            │  Subcommand detected  │  │
│  │  (or .kaku arg)│            │  (new/draw/preview/…) │  │
│  └──────┬───────┘            └──────────┬────────────┘  │
│         │                               │               │
│         ▼                               ▼               │
│  ┌──────────────┐            ┌───────────────────────┐  │
│  │   TUI Path    │            │      CLI Path         │  │
│  │  (existing)   │            │   cli/mod.rs          │  │
│  │  app.rs       │            │   cli/draw.rs         │  │
│  │  input.rs     │            │   cli/preview.rs      │  │
│  │  ui/          │            │   cli/inspect.rs      │  │
│  └──────┬───────┘            │   cli/diff.rs         │  │
│         │                    │   cli/stats.rs        │  │
│         │                    │   cli/history_cmd.rs   │  │
│         │                    │   cli/palette_cmd.rs   │  │
│         │                    └──────────┬────────────┘  │
│         │                               │               │
│         ▼                               ▼               │
│  ┌─────────────────────────────────────────────────────┐│
│  │              Shared Core Modules                     ││
│  │  canvas.rs  cell.rs  tools.rs  symmetry.rs          ││
│  │  history.rs  project.rs  export.rs  palette.rs      ││
│  │  theme.rs  + NEW: oplog.rs                          ││
│  └─────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────┘
```

### 2.2 Design Principles

1. **No wrapper layer**: The CLI commands call existing modules directly. No `ops/` abstraction — the existing modules ARE the shared API.
2. **Zero TUI impact**: CLI code lives in `cli/`, never touches `app.rs`, `input.rs`, or `ui/`. The TUI path remains byte-for-byte identical.
3. **Stateless commands**: Each CLI invocation loads a `.kaku` file, performs an operation, writes back. No persistent process.
4. **One new module**: `oplog.rs` handles CLI operation logging (undo/redo). This is the only new shared module.

## 3. Technology Stack

| Component | Choice | Justification |
|-----------|--------|---------------|
| CLI parsing | `clap` v4 (derive) | Standard Rust CLI framework. Derive macros minimize boilerplate. |
| Serialization | `serde` + `serde_json` (existing) | Already used for `.kaku` files. Reused for JSON output and operation logs. |
| TUI framework | `ratatui` + `crossterm` (existing) | Unchanged. Only compiled when TUI path is used. |
| Clipboard | `arboard` (existing) | Unchanged. Not used by CLI path. |

### 3.1 New Dependency

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
```

This is the ONLY new dependency. Binary size impact: ~200KB (clap is well-optimized with derive-only features).

## 4. Component Design

### 4.1 Entry Point — `main.rs`

Current `main.rs` immediately initializes the terminal (raw mode, alternate screen). This must change to check for CLI subcommands first.

**New flow:**

```rust
// main.rs (new structure)
mod app;
mod canvas;
mod cell;
mod cli;        // NEW
mod export;
mod history;
mod input;
mod oplog;      // NEW
mod palette;
mod project;
mod symmetry;
mod theme;
mod tools;
mod ui;

use clap::Parser;

fn main() -> std::io::Result<()> {
    let args = cli::Cli::parse();

    match args.command {
        Some(cmd) => cli::run(cmd),    // CLI path — no terminal init
        None => run_tui(args.file),    // TUI path — existing behavior
    }
}

fn run_tui(file: Option<String>) -> std::io::Result<()> {
    // Existing main.rs code moves here verbatim
}
```

**Key constraint**: `cli::run()` must never call `enable_raw_mode()`, `EnterAlternateScreen`, or any crossterm/ratatui function. It operates purely on files and stdout.

### 4.2 CLI Argument Parser — `cli/mod.rs`

```rust
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "kakukuma", about = "Terminal ANSI art editor")]
pub struct Cli {
    /// Open .kaku file in TUI editor
    pub file: Option<String>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Create a new .kaku project file
    New {
        file: String,
        #[arg(long, default_value_t = 48)]
        width: usize,
        #[arg(long, default_value_t = 32)]
        height: usize,
        #[arg(long, value_parser = parse_size)]
        size: Option<(usize, usize)>,
        #[arg(long)]
        force: bool,
    },

    /// Draw on canvas using a tool
    Draw {
        #[command(subcommand)]
        tool: DrawTool,
    },

    /// Render canvas to stdout
    Preview {
        file: String,
        #[arg(long, default_value = "ansi")]
        format: PreviewFormat,
        #[arg(long, value_parser = parse_region)]
        region: Option<(usize, usize, usize, usize)>,
        #[arg(long, default_value = "truecolor")]
        color_format: CliColorFormat,
    },

    /// Query canvas cell data
    Inspect {
        file: String,
        #[arg(value_parser = parse_coord)]
        coord: Option<(usize, usize)>,
        #[arg(long, value_parser = parse_region)]
        region: Option<(usize, usize, usize, usize)>,
        #[arg(long)]
        row: Option<usize>,
        #[arg(long)]
        col: Option<usize>,
    },

    /// Export canvas to file
    Export {
        file: String,
        #[arg(long)]
        output: String,
        #[arg(long, default_value = "ansi")]
        format: PreviewFormat,
        #[arg(long, default_value = "truecolor")]
        color_format: CliColorFormat,
    },

    /// Compare two canvas files
    Diff {
        file1: String,
        file2: Option<String>,
        #[arg(long)]
        before: bool,
    },

    /// Canvas statistics
    Stats { file: String },

    /// Undo last CLI operation
    Undo {
        file: String,
        #[arg(long, default_value_t = 1)]
        count: usize,
    },

    /// Redo last undone CLI operation
    Redo {
        file: String,
        #[arg(long, default_value_t = 1)]
        count: usize,
    },

    /// Show operation log
    History {
        file: String,
        #[arg(long)]
        full: bool,
    },

    /// Palette management
    Palette {
        #[command(subcommand)]
        action: PaletteAction,
    },
}

#[derive(Subcommand)]
pub enum DrawTool {
    Pencil {
        file: String,
        #[arg(value_parser = parse_coord)]
        coord: (usize, usize),
        #[command(flatten)]
        opts: DrawOpts,
    },
    Eraser {
        file: String,
        #[arg(value_parser = parse_coord)]
        coord: (usize, usize),
        #[arg(long, value_parser = parse_region)]
        region: Option<(usize, usize, usize, usize)>,
    },
    Line {
        file: String,
        #[arg(value_parser = parse_coord)]
        from: (usize, usize),
        #[arg(value_parser = parse_coord)]
        to: (usize, usize),
        #[command(flatten)]
        opts: DrawOpts,
    },
    Rect {
        file: String,
        #[arg(value_parser = parse_coord)]
        from: (usize, usize),
        #[arg(value_parser = parse_coord)]
        to: (usize, usize),
        #[arg(long)]
        filled: bool,
        #[command(flatten)]
        opts: DrawOpts,
    },
    Fill {
        file: String,
        #[arg(value_parser = parse_coord)]
        coord: (usize, usize),
        #[command(flatten)]
        opts: DrawOpts,
    },
    Eyedropper {
        file: String,
        #[arg(value_parser = parse_coord)]
        coord: (usize, usize),
    },
}

#[derive(clap::Args)]
pub struct DrawOpts {
    #[arg(long)]
    pub color: Option<String>,
    #[arg(long)]
    pub fg: Option<String>,
    #[arg(long)]
    pub bg: Option<String>,
    #[arg(long, name = "char")]
    pub ch: Option<char>,
    #[arg(long, default_value = "off")]
    pub symmetry: CliSymmetry,
    #[arg(long)]
    pub no_log: bool,
}

#[derive(ValueEnum, Clone)]
pub enum PreviewFormat { Ansi, Json }

#[derive(ValueEnum, Clone)]
pub enum CliColorFormat { Truecolor, Color256, Color16 }

#[derive(ValueEnum, Clone)]
pub enum CliSymmetry { Off, Horizontal, Vertical, Quad }

#[derive(Subcommand)]
pub enum PaletteAction {
    List,
    Show { name: String },
    Create { name: String, file: String },
    Export { name: String, #[arg(long)] output: String },
    Add { name: String, color: String },
    Themes,
    Theme { name: String },
}
```

### 4.3 Draw Command — `cli/draw.rs`

This is the most complex CLI handler. It composes existing modules.

**Flow for each draw command:**

```
1. Load Project from .kaku file (project::Project::load_from_file)
2. Parse color args into Option<Rgb> (cell::parse_hex_color)
3. Parse block char (or use default '█')
4. Call tool function (tools::pencil/line/rectangle/flood_fill/eraser)
   → Returns Vec<CellMutation>
5. Apply symmetry (symmetry::apply_symmetry)
   → Returns expanded Vec<CellMutation>
6. Apply mutations to canvas (canvas.set for each mutation)
7. Log operation to .kaku.log (oplog::append, unless --no-log)
8. Save project atomically (write to .kaku.tmp, rename to .kaku)
9. Print summary to stdout (JSON: {"cells_modified": N})
```

**Color resolution logic:**

```rust
fn resolve_colors(opts: &DrawOpts) -> (Option<Rgb>, Option<Rgb>) {
    let fg = opts.fg.as_deref()
        .or(opts.color.as_deref())
        .and_then(|s| parse_hex_color(s));
    let bg = opts.bg.as_deref()
        .and_then(|s| parse_hex_color(s));
    // Default fg to white if no color specified at all
    let fg = fg.or(Some(Rgb::WHITE));
    (fg, bg)
}
```

**Symmetry mapping:**

```rust
fn to_symmetry_mode(s: &CliSymmetry) -> SymmetryMode {
    match s {
        CliSymmetry::Off => SymmetryMode::Off,
        CliSymmetry::Horizontal => SymmetryMode::Horizontal,
        CliSymmetry::Vertical => SymmetryMode::Vertical,
        CliSymmetry::Quad => SymmetryMode::Quad,
    }
}
```

**Atomic file write:**

```rust
fn atomic_save(project: &mut Project, path: &Path) -> Result<(), String> {
    let tmp = path.with_extension("kaku.tmp");
    project.save_to_file(&tmp)?;
    std::fs::rename(&tmp, path)
        .map_err(|e| format!("Rename error: {}", e))
}
```

### 4.4 Operation Log — `oplog.rs` (New Module)

The operation log enables CLI undo/redo. It is a JSON Lines file (`.kaku.log`) where each line is one operation entry.

**Data structures:**

```rust
use serde::{Deserialize, Serialize};
use crate::cell::{Cell, Rgb};

const MAX_LOG_ENTRIES: usize = 256;

#[derive(Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub command: String,
    pub mutations: Vec<LogMutation>,
}

#[derive(Serialize, Deserialize)]
pub struct LogMutation {
    pub x: usize,
    pub y: usize,
    pub old: LogCell,
    pub new: LogCell,
}

#[derive(Serialize, Deserialize)]
pub struct LogCell {
    pub ch: char,
    pub fg: Option<Rgb>,
    pub bg: Option<Rgb>,
}
```

**Key operations:**

```rust
/// Derive log path from .kaku path: "art.kaku" → "art.kaku.log"
pub fn log_path(kaku_path: &Path) -> PathBuf {
    let mut p = kaku_path.to_path_buf().into_os_string();
    p.push(".log");
    PathBuf::from(p)
}

/// Append an entry to the operation log.
/// Truncates redo entries (entries after undo pointer) and prunes to MAX_LOG_ENTRIES.
pub fn append(path: &Path, entry: LogEntry) -> Result<(), String>;

/// Read all log entries.
pub fn read_log(path: &Path) -> Result<Vec<LogEntry>, String>;

/// Pop the last N entries for undo. Returns the popped entries.
/// Moves the undo pointer rather than deleting (enables redo).
pub fn pop_for_undo(path: &Path, count: usize) -> Result<Vec<LogEntry>, String>;

/// Restore the last N undone entries for redo.
pub fn push_for_redo(path: &Path, count: usize) -> Result<Vec<LogEntry>, String>;
```

**Log file format** (JSON Lines — one JSON object per line):

```
{"timestamp":"2026-02-14T10:30:00Z","command":"draw pencil 5,5","mutations":[...]}
{"timestamp":"2026-02-14T10:30:05Z","command":"draw line 0,0 10,10","mutations":[...]}
```

**Undo pointer**: The log file tracks the undo boundary. When `undo` is called, entries are not deleted — they're marked as "undone" by moving a pointer stored as the first line of the file:

```
{"pointer":1,"total":3}
{"timestamp":"...","command":"draw pencil 5,5","mutations":[...]}
{"timestamp":"...","command":"draw line 0,0 10,10","mutations":[...]}  ← undone
{"timestamp":"...","command":"draw rect 2,2 8,8","mutations":[...]}   ← undone
```

Active entries: lines 1 through `pointer`. Undone entries: lines `pointer+1` through `total`. A new `append` clears all undone entries (same as TUI behavior).

### 4.5 Preview Command — `cli/preview.rs`

**ANSI format**: Directly calls existing `export::to_ansi()` with the appropriate `ColorFormat`, then writes to stdout.

**JSON format**: Custom serialization of canvas state.

```rust
#[derive(Serialize)]
struct JsonPreview {
    width: usize,
    height: usize,
    cells: Vec<Vec<JsonCell>>,
    non_empty_count: usize,
}

#[derive(Serialize)]
struct JsonCell {
    x: usize,
    y: usize,
    fg: Option<String>,   // "#RRGGBB" or null
    bg: Option<String>,
    #[serde(rename = "char")]
    ch: String,           // The Unicode character
}
```

**Region support**: If `--region x1,y1,x2,y2` is specified, only cells within that bounding box are included in output.

### 4.6 Inspect Command — `cli/inspect.rs`

Four modes based on arguments:

| Arguments | Behavior |
|-----------|----------|
| `5,5` (coordinate) | Return single cell as JSON object |
| `--region 0,0,10,10` | Return non-empty cells in region as JSON array |
| `--row 5` | Return all cells in row 5 as JSON array |
| `--col 5` | Return all cells in column 5 as JSON array |

**Output format** (single cell):
```json
{"x": 5, "y": 5, "fg": "#FF6600", "bg": null, "char": "█", "empty": false}
```

### 4.7 Diff Command — `cli/diff.rs`

**Two-file diff** (`kakukuma diff a.kaku b.kaku`):

```rust
fn diff_canvases(a: &Canvas, b: &Canvas) -> DiffResult {
    // Handle different dimensions: use max dimensions, treat missing cells as empty
    let w = a.width.max(b.width);
    let h = a.height.max(b.height);
    let mut changes = Vec::new();
    let (mut added, mut removed, mut modified, mut unchanged) = (0, 0, 0, 0);

    for y in 0..h {
        for x in 0..w {
            let cell_a = a.get(x, y).unwrap_or(Cell::default());
            let cell_b = b.get(x, y).unwrap_or(Cell::default());
            if cell_a != cell_b {
                let a_empty = cell_a.is_empty();
                let b_empty = cell_b.is_empty();
                match (a_empty, b_empty) {
                    (true, false) => added += 1,
                    (false, true) => removed += 1,
                    _ => modified += 1,
                }
                changes.push(DiffChange { x, y, before: cell_a, after: cell_b });
            } else {
                unchanged += 1;
            }
        }
    }

    DiffResult { changes, added, removed, modified, unchanged }
}
```

**Before-last-operation diff** (`kakukuma diff art.kaku --before`):

Reads the last log entry from `.kaku.log` and reconstructs the "before" state by applying the inverse mutations.

### 4.8 Stats Command — `cli/stats.rs`

```rust
#[derive(Serialize)]
struct CanvasStats {
    canvas: CanvasDimensions,
    fill: FillStats,
    colors: ColorStats,
    characters: CharStats,
    bounding_box: Option<BoundingBox>,
    symmetry_score: SymmetryScore,
}

#[derive(Serialize)]
struct SymmetryScore {
    horizontal: f64,   // 0.0 to 1.0
    vertical: f64,
}
```

**Symmetry score algorithm**:

```rust
fn symmetry_score_horizontal(canvas: &Canvas) -> f64 {
    let mut matching = 0usize;
    let mut total = 0usize;

    for y in 0..canvas.height {
        for x in 0..canvas.width / 2 {
            let mx = canvas.width - 1 - x;
            let left = canvas.get(x, y).unwrap_or(Cell::default());
            let right = canvas.get(mx, y).unwrap_or(Cell::default());
            total += 1;
            if left == right {
                matching += 1;
            }
        }
    }

    if total == 0 { 0.0 } else { matching as f64 / total as f64 }
}
```

Compares each cell with its mirror counterpart. Score of 1.0 = perfectly symmetric. Only non-empty cells contribute (empty-vs-empty is a match).

### 4.9 History Command — `cli/history_cmd.rs`

Reads the operation log and displays entries.

**Summary mode** (default):
```
# Operation History (3 entries, 1 undone)
  1. [2026-02-14T10:30:00Z] draw pencil 5,5 (1 cell)
  2. [2026-02-14T10:30:05Z] draw line 0,0 10,10 (11 cells)
  * 3. [2026-02-14T10:30:10Z] draw rect 2,2 8,8 (24 cells) [undone]
```

**Full mode** (`--full`): Includes mutation details as JSON for each entry.

### 4.10 Palette Command — `cli/palette_cmd.rs`

Wraps existing `palette.rs` functions:

| Subcommand | Implementation |
|------------|----------------|
| `list` | `palette::list_palette_files()` on current directory + `~/.kakukuma/palettes/` |
| `show NAME` | Load palette, print colors as JSON array of `{"index": N, "hex": "#RRGGBB", "swatch": "██"}` |
| `create NAME FILE` | Scan canvas for unique colors, save as `.palette` |
| `export NAME --output FILE` | Load palette, write to specified path |
| `add NAME COLOR` | Load palette, append color, save |
| `themes` | List theme names from `theme.rs` constants |
| `theme NAME` | Print theme colors as JSON |

### 4.11 New Canvas Command — `cli/new.rs` (part of `cli/mod.rs`)

```rust
fn cmd_new(file: &str, width: usize, height: usize, force: bool) -> io::Result<()> {
    let path = Path::new(file);
    if path.exists() && !force {
        eprintln!("Error: '{}' already exists. Use --force to overwrite.", file);
        std::process::exit(1);
    }

    let canvas = Canvas::new_with_size(width, height);
    let mut project = Project::new(
        path.file_stem().and_then(|s| s.to_str()).unwrap_or("untitled"),
        canvas,
        Rgb::WHITE,
        SymmetryMode::Off,
    );

    project.save_to_file(path)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    // Initialize empty log
    let log = oplog::log_path(path);
    oplog::init_log(&log)?;

    let json = serde_json::json!({
        "created": file,
        "width": width,
        "height": height,
    });
    println!("{}", serde_json::to_string(&json).unwrap());
    Ok(())
}
```

## 5. Data Architecture

### 5.1 Existing Data Model (Unchanged)

```
Project (v5)
├── version: u32 = 5
├── name: String
├── created_at: String (ISO8601)
├── modified_at: String (ISO8601)
├── color: Rgb [r, g, b]
├── symmetry: SymmetryMode
└── canvas: Canvas
    ├── width: usize
    ├── height: usize
    └── cells: Vec<Vec<Cell>>
        └── Cell
            ├── ch: char
            ├── fg: Option<Rgb>
            └── bg: Option<Rgb>
```

### 5.2 New Data: Operation Log (`.kaku.log`)

```
LogFile (JSON Lines)
├── Line 0: LogHeader
│   ├── pointer: usize (index of last active entry)
│   └── total: usize (total entries including undone)
└── Lines 1..N: LogEntry
    ├── timestamp: String (ISO8601)
    ├── command: String (human-readable command description)
    └── mutations: Vec<LogMutation>
        ├── x: usize
        ├── y: usize
        ├── old: LogCell { ch, fg, bg }
        └── new: LogCell { ch, fg, bg }
```

### 5.3 File Naming Convention

| File | Purpose |
|------|---------|
| `art.kaku` | Project file (existing format, unchanged) |
| `art.kaku.log` | CLI operation log (new, JSON Lines) |
| `art.kaku.tmp` | Temporary file during atomic write (deleted after rename) |
| `art.kaku.autosave` | TUI auto-save (existing, unchanged) |

## 6. CLI Output Contracts

All CLI commands that produce output write to stdout as valid JSON. Errors go to stderr.

### 6.1 Draw Command Output

```json
{"ok": true, "cells_modified": 11, "tool": "line", "symmetry": "off"}
```

### 6.2 Eyedropper Output

```json
{"x": 5, "y": 5, "fg": "#FF6600", "bg": null, "char": "█"}
```

### 6.3 Preview JSON Output

```json
{
  "width": 48, "height": 32,
  "cells": [[{"x":0,"y":0,"fg":null,"bg":null,"char":" "}, ...]],
  "non_empty_count": 42
}
```

### 6.4 Inspect Output

```json
{"x": 5, "y": 5, "fg": "#FF6600", "bg": null, "char": "█", "empty": false}
```

### 6.5 Diff Output

```json
{
  "changes": [{"x":5,"y":5,"before":{"fg":"#FF0000","bg":null,"char":"█"},"after":{"fg":"#00FF00","bg":null,"char":"█"}}],
  "added": 3, "removed": 1, "modified": 7, "unchanged": 1525
}
```

### 6.6 Stats Output

```json
{
  "canvas": {"width": 48, "height": 32, "total_cells": 1536},
  "fill": {"empty": 1494, "filled": 42, "fill_percent": 2.73},
  "colors": {
    "unique_fg": 5, "unique_bg": 2,
    "distribution": [{"color": "#FF6600", "count": 15, "percent": 35.71}]
  },
  "characters": {
    "unique": 3,
    "distribution": [{"char": "█", "count": 30, "percent": 71.43}]
  },
  "bounding_box": {"min_x": 2, "min_y": 3, "max_x": 20, "max_y": 15},
  "symmetry_score": {"horizontal": 0.85, "vertical": 0.42}
}
```

### 6.7 Error Output (stderr)

```json
{"error": "coordinates_out_of_bounds", "message": "Position (150, 5) exceeds canvas dimensions (48x32)"}
```

Plain text errors (default): `Error: Position (150, 5) exceeds canvas dimensions (48x32)`

## 7. Module Dependency Graph

```
main.rs
├── cli/mod.rs (clap parsing)
│   ├── cli/draw.rs
│   │   ├── canvas.rs
│   │   ├── cell.rs (parse_hex_color, Rgb, blocks)
│   │   ├── tools.rs (pencil, eraser, line, rectangle, flood_fill, eyedropper)
│   │   ├── symmetry.rs (apply_symmetry)
│   │   ├── project.rs (load_from_file, save_to_file)
│   │   └── oplog.rs (append)
│   ├── cli/preview.rs
│   │   ├── export.rs (to_ansi, to_plain_text)
│   │   └── project.rs
│   ├── cli/inspect.rs
│   │   ├── canvas.rs
│   │   └── project.rs
│   ├── cli/diff.rs
│   │   ├── canvas.rs, cell.rs
│   │   ├── project.rs
│   │   └── oplog.rs
│   ├── cli/stats.rs
│   │   ├── canvas.rs, cell.rs
│   │   └── project.rs
│   ├── cli/history_cmd.rs
│   │   └── oplog.rs
│   └── cli/palette_cmd.rs
│       ├── palette.rs
│       └── theme.rs
├── app.rs (TUI only)
│   ├── canvas.rs, cell.rs, tools.rs, symmetry.rs
│   ├── history.rs, project.rs, export.rs, palette.rs, theme.rs
│   └── input.rs → ui/
└── oplog.rs (new, shared)
    ├── cell.rs (Rgb, Cell)
    └── project.rs (now_iso8601 — needs to be made pub)
```

## 8. Source File Layout

```
src/
├── main.rs             MODIFIED — add clap routing, extract run_tui()
├── cli/
│   ├── mod.rs          NEW — Cli struct, Command enum, subcommand routing
│   ├── draw.rs         NEW — draw subcommand handlers
│   ├── preview.rs      NEW — preview and export handlers
│   ├── inspect.rs      NEW — cell/region inspection
│   ├── diff.rs         NEW — canvas comparison
│   ├── stats.rs        NEW — canvas statistics
│   ├── history_cmd.rs  NEW — operation log display
│   └── palette_cmd.rs  NEW — palette management
├── oplog.rs            NEW — operation log read/write
├── app.rs              UNCHANGED
├── canvas.rs           UNCHANGED
├── cell.rs             UNCHANGED
├── export.rs           UNCHANGED (pub fn already exposed)
├── history.rs          UNCHANGED (CellMutation reused by oplog)
├── input.rs            UNCHANGED
├── palette.rs          UNCHANGED (pub fn already exposed)
├── project.rs          MINOR — make now_iso8601() pub(crate) for oplog
├── symmetry.rs         UNCHANGED
├── theme.rs            MINOR — expose theme data as pub constants
└── ui/
    ├── mod.rs           UNCHANGED
    ├── editor.rs        UNCHANGED
    ├── toolbar.rs       UNCHANGED
    ├── palette.rs       UNCHANGED
    └── statusbar.rs     UNCHANGED
```

**File count**: 10 new files, 2 minor modifications, 13 unchanged.

## 9. Coordinate Parsing

CLI coordinates use `X,Y` format (no spaces):

```rust
fn parse_coord(s: &str) -> Result<(usize, usize), String> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 2 {
        return Err(format!("Expected X,Y format, got '{}'", s));
    }
    let x = parts[0].parse::<usize>().map_err(|_| format!("Invalid X: '{}'", parts[0]))?;
    let y = parts[1].parse::<usize>().map_err(|_| format!("Invalid Y: '{}'", parts[1]))?;
    Ok((x, y))
}

fn parse_region(s: &str) -> Result<(usize, usize, usize, usize), String> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 4 {
        return Err(format!("Expected X1,Y1,X2,Y2 format, got '{}'", s));
    }
    // parse all four...
}

fn parse_size(s: &str) -> Result<(usize, usize), String> {
    let parts: Vec<&str> = s.split('x').collect();
    // "32x24" → (32, 24)
}
```

## 10. Error Handling Strategy

### 10.1 Exit Codes

| Code | Meaning | Examples |
|------|---------|---------|
| 0 | Success | Operation completed |
| 1 | User error | Bad args, file not found, coords out of bounds, file already exists |
| 2 | Internal error | File corruption, I/O failure, serialization error |

### 10.2 Error Types

```rust
enum CliError {
    FileNotFound(String),
    FileAlreadyExists(String),
    CoordinatesOutOfBounds { x: usize, y: usize, width: usize, height: usize },
    InvalidColor(String),
    InvalidCoordinate(String),
    ProjectLoadError(String),
    ProjectSaveError(String),
    LogError(String),
}

impl CliError {
    fn exit_code(&self) -> i32 {
        match self {
            CliError::FileNotFound(_) | CliError::FileAlreadyExists(_)
            | CliError::CoordinatesOutOfBounds { .. } | CliError::InvalidColor(_)
            | CliError::InvalidCoordinate(_) => 1,
            _ => 2,
        }
    }

    fn to_json(&self) -> serde_json::Value {
        // Structured JSON error
    }
}
```

### 10.3 Error Reporting

```rust
fn handle_error(e: CliError) -> ! {
    eprintln!("Error: {}", e);
    std::process::exit(e.exit_code());
}
```

## 11. Performance Design

### 11.1 Hot Path Analysis

The CLI hot path is: **parse args → load .kaku → mutate → save .kaku → write stdout**.

| Phase | Expected Cost | Optimization |
|-------|---------------|-------------|
| Clap parse | ~1ms | Negligible |
| Load .kaku (48x32) | ~5ms | JSON parse of ~50KB file |
| Load .kaku (128x128) | ~30ms | JSON parse of ~500KB file |
| Tool execution | ~1ms | Already optimized (tools.rs) |
| Save .kaku | ~5-30ms | Pretty-print JSON |
| Log append | ~1ms | Single line append |
| **Total (48x32)** | **~15ms** | Well under 100ms target |
| **Total (128x128)** | **~65ms** | Under 200ms target |

### 11.2 Optimization: Compact Save for CLI

For CLI operations, we can use `serde_json::to_string()` (compact) instead of `to_string_pretty()` to halve file size and save time. The project format remains identical — just whitespace differs.

However, to maintain human-readability of .kaku files (they're also opened in editors), we keep pretty-printing as default. A `--compact` flag could be added later if performance becomes an issue.

### 11.3 Large Canvas Optimization

For canvases at MAX_DIMENSION (128x128 = 16,384 cells), JSON serialization dominates. If this becomes a bottleneck:
- **Phase 1** (this cycle): Accept the ~65ms cost. It's well under targets.
- **Phase 2** (future): Consider binary format (MessagePack) as an optional save format.

## 12. Security Considerations

### 12.1 Path Traversal

All file operations use the path provided by the user directly. Since this is a local CLI tool (not a server), path traversal is not a security concern — the user already has filesystem access.

### 12.2 Input Validation

| Input | Validation |
|-------|-----------|
| Coordinates | Must be within canvas bounds (0 to width-1, 0 to height-1) |
| Colors | Must match `#RRGGBB` or `RRGGBB` hex format |
| Canvas dimensions | Clamped to 8-128 (existing `Canvas::new_with_size`) |
| File paths | Standard filesystem access — no special sanitization needed |
| Unicode characters | Accepted as-is — `--char` flag passes through to `Cell.ch` |

### 12.3 Concurrent Access

**Constraint**: Single-writer. The CLI does not implement file locking. If two CLI processes write to the same `.kaku` file simultaneously, the last write wins. The atomic write (tmp + rename) prevents corruption but not data loss.

This is acceptable for the agent use case (single agent per file).

## 13. Testing Strategy

### 13.1 Unit Tests

Each new module gets unit tests inline (`#[cfg(test)]`):

| Module | Tests |
|--------|-------|
| `cli/draw.rs` | Color parsing, coordinate validation, tool dispatch |
| `cli/preview.rs` | JSON output format, region filtering, ANSI passthrough |
| `cli/inspect.rs` | Single cell, region, row, column queries |
| `cli/diff.rs` | Same canvas, different canvas, different dimensions, before-last |
| `cli/stats.rs` | Empty canvas, partially filled, symmetry scoring |
| `oplog.rs` | Append, read, undo pointer, redo, pruning at 256 |
| `cli/palette_cmd.rs` | List, show, create from canvas |

### 13.2 Integration Tests

Located in `tests/` (Cargo integration test directory):

```
tests/
├── cli_new.rs        Create canvas, verify file
├── cli_draw.rs       Draw with each tool, verify canvas state
├── cli_preview.rs    Preview ANSI and JSON, verify output
├── cli_roundtrip.rs  Draw → preview → inspect → verify consistency
├── cli_undo_redo.rs  Draw → undo → redo → verify state
├── cli_diff.rs       Create two files, diff, verify output
├── cli_stats.rs      Draw patterns, verify statistics
└── cli_symmetry.rs   Draw with symmetry flags, verify mirrored cells
```

Integration tests invoke the binary via `std::process::Command` and check stdout/stderr/exit codes.

### 13.3 Regression Tests

The existing `cargo test` suite (65+ tests across canvas, cell, tools, history, symmetry, export, project) continues to run and must pass. No existing test is modified.

## 14. Development Workflow

### 14.1 Implementation Order

The implementation follows a dependency-driven order:

1. **Foundation**: `oplog.rs` (no dependencies on CLI)
2. **CLI scaffold**: `cli/mod.rs` with clap structs, `main.rs` routing
3. **New canvas**: `cli/new.rs` (simplest command, validates scaffold)
4. **Draw commands**: `cli/draw.rs` (core value — enables agent creation)
5. **Preview**: `cli/preview.rs` (enables agent feedback loop)
6. **Inspect**: `cli/inspect.rs` (enables agent state queries)
7. **Undo/Redo**: `cli/history_cmd.rs` (depends on oplog)
8. **Diff**: `cli/diff.rs` (depends on oplog for `--before`)
9. **Stats**: `cli/stats.rs` (independent)
10. **Palette**: `cli/palette_cmd.rs` (lowest priority)

### 14.2 Existing Module Changes

Only two files need minor modifications:

**`project.rs`**: Make `now_iso8601()` visible to `oplog.rs`:
```rust
// Change from:
fn now_iso8601() -> String { ... }
// To:
pub(crate) fn now_iso8601() -> String { ... }
```

**`theme.rs`**: Expose theme data for CLI palette command:
```rust
// Add pub accessor:
pub fn all_themes() -> &'static [Theme] {
    &[WARM, NEON, DARK]
}
```

## 15. Technical Risks & Mitigations

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| clap conflicts with existing arg parsing | Can't route between TUI and CLI | Low | clap handles `None` subcommand gracefully; test early |
| Operation log corruption | Undo fails | Low | Validate log on read; rebuild from scratch if corrupt |
| Large flood fill in CLI | Slow for 128x128 canvas | Low | Existing flood_fill is O(n) with visited set; fast enough |
| Tool behavior drift | CLI and TUI produce different results | Medium | Both call the same `tools::` functions; shared tests |
| JSON output breaking changes | Agent code breaks on format change | Medium | Document output contracts in SDD; version the output schema |

## 16. Future Considerations

| Feature | When | Notes |
|---------|------|-------|
| `--batch` flag for draw | Cycle 2 | Accept multiple draw commands via stdin |
| `kakukuma watch` | Cycle 2 | Monitor .kaku file and re-render on change |
| MCP Server | Cycle 3 | Wrap CLI operations as MCP tools |
| Binary save format | If needed | MessagePack for large canvas performance |
| `--quiet` flag | Cycle 2 | Suppress stdout output for scripting |
