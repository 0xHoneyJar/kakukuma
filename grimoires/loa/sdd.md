# SDD: Creative Power Tools — Command Palette, Reference Layer, Batch Draw

> **Cycle**: 018
> **Created**: 2026-02-28
> **Status**: Draft
> **PRD**: grimoires/loa/prd.md

---

## 1. Executive Summary

Add three features to Kakukuma that unlock human discoverability (command palette), artist productivity (reference layer), and agent throughput (batch draw). All three build on the existing architecture without structural changes — they add new AppMode variants, new CLI commands, and new rendering logic, but do not modify existing module boundaries or data flows.

**Key constraint**: Only `Color::Indexed(n)` — never `Color::Rgb()`. The reference layer renders via the same `Rgb::to_ratatui()` path as all other colors.

---

## 2. System Architecture

### Current Architecture (post-cycle-017)

```
lib.rs (11 public modules)
  ├── canvas.rs, cell.rs, export.rs, history.rs, import.rs
  ├── oplog.rs, palette.rs, project.rs, symmetry.rs, theme.rs, tools.rs

main.rs (binary — 4 modules + re-exports)
  ├── app.rs          (App struct, AppMode enum, state machine)
  ├── cli/            (CLI subcommands — draw, inspect, preview, etc.)
  ├── input.rs        (crossterm event dispatch, CanvasArea)
  └── ui/             (ratatui rendering — editor, toolbar, palette, statusbar, dialogs)
```

### Changes for Cycle 018

```
lib.rs — UNCHANGED (no new public modules)

main.rs modules:
  app.rs     — ADD: AppMode::CommandPalette, reference/palette state fields
  cli/mod.rs — ADD: Command::Batch, Command::Reference
  cli/batch.rs — NEW: batch JSON parser and executor
  input.rs   — ADD: handle_command_palette(), Spacebar dispatch
  ui/mod.rs  — ADD: render_command_palette(), render overlays
  ui/editor.rs — MODIFY: reference layer rendering behind canvas cells
```

No changes to lib.rs modules. The batch command reuses existing `tools::*` functions from the library. The reference layer uses existing `import.rs` scaling logic for image loading.

---

## 3. Component Design

### 3.1 Command Palette (FR-1)

#### 3.1.1 Command Registry

A static registry of all editor commands. Each entry maps a name, category, keyboard shortcut hint, and an action closure.

```rust
// src/app.rs (new types)

pub struct PaletteCommand {
    pub name: &'static str,
    pub category: &'static str,
    pub shortcut: &'static str,
    pub action: fn(&mut App),
}
```

The registry is a `const` array built at compile time. ~30 commands across 9 categories:

| Category | Commands | Count |
|----------|----------|-------|
| Tools | Pencil, Eraser, Line, Rectangle, Fill, Eyedropper | 6 |
| Canvas | New Canvas, Resize Canvas, Clear Canvas, Import Image | 4 |
| File | Save, Save As, Open, Export | 4 |
| Edit | Undo, Redo | 2 |
| View | Zoom In, Zoom Out, Toggle Grid, Cycle Theme | 4 |
| Character | Block Picker, Shade Cycle, Character Input | 3 |
| Color | Hex Color Input, Color Sliders | 2 |
| Symmetry | Symmetry Off, Horizontal, Vertical, Quad | 4 |
| Help | Show Help, Quit | 2 |

**Total**: ~31 commands

Each `action` is a simple function pointer: `fn(&mut App)`. Example:
```rust
PaletteCommand {
    name: "Pencil",
    category: "Tools",
    shortcut: "P",
    action: |app| { app.active_tool = ToolKind::Pencil; app.cancel_tool(); },
}
```

#### 3.1.2 Fuzzy Matching

Simple substring-based fuzzy match — no external crate needed.

```rust
fn fuzzy_match(query: &str, target: &str) -> bool {
    let query = query.to_lowercase();
    let target = target.to_lowercase();
    // All query chars must appear in target in order
    let mut target_chars = target.chars();
    for qc in query.chars() {
        if qc == ' ' { continue; } // spaces are separators, skip
        loop {
            match target_chars.next() {
                Some(tc) if tc == qc => break,
                Some(_) => continue,
                None => return false,
            }
        }
    }
    true
}
```

This handles "sav" → "Save", "sym h" → "Symmetry Horizontal" naturally.

#### 3.1.3 App State

New fields in `App` struct:

```rust
// Command palette state
pub palette_query: String,           // Current search text
pub palette_filtered: Vec<usize>,    // Indices into COMMANDS registry
pub palette_selected: usize,         // Cursor in filtered list
```

New `AppMode` variant:

```rust
AppMode::CommandPalette,
```

#### 3.1.4 Input Handling

In `input.rs`, the Spacebar dispatch changes:

```rust
KeyCode::Char(' ') => {
    if app.canvas_cursor_active {
        // Existing behavior: draw at canvas cursor
        let (x, y) = app.canvas_cursor;
        // ... existing draw logic ...
    } else {
        // NEW: Open command palette
        app.palette_query.clear();
        app.palette_selected = 0;
        app.palette_filtered = (0..COMMANDS.len()).collect();
        app.mode = AppMode::CommandPalette;
    }
}
```

New handler `handle_command_palette(app, key)`:
- **Printable chars**: Append to `palette_query`, re-filter
- **Backspace**: Remove last char from `palette_query`, re-filter
- **Up/Down arrows**: Navigate `palette_selected`
- **Enter**: Execute selected command's `action`, return to Normal
- **Esc**: Dismiss, return to Normal

#### 3.1.5 Rendering

New function in `ui/mod.rs`: `render_command_palette(f, app, size)`.

Layout: Centered overlay, top-third of screen, 50 chars wide, up to 12 rows.

```
┌─────────── Command Palette ───────────┐
│ > sym h_                              │
│                                       │
│   Symmetry Horizontal       Ctrl+1    │
│ → Symmetry Vertical         Ctrl+2    │ ← selected
│   Symmetry Quad             Ctrl+3    │
│   Symmetry Off              Ctrl+0    │
└───────────────────────────────────────┘
```

- First line: text input with cursor
- Filtered commands below, max ~10 visible
- Selected item highlighted with `theme.highlight`
- Shortcut hints right-aligned in `theme.dim`
- Theme-aware: uses `app.theme()` for all colors

---

### 3.2 Reference Layer (FR-2)

#### 3.2.1 Data Model

The reference layer is stored as pre-processed cell data, not a raw image. On load, the image is converted to a grid of `Rgb` background colors at canvas resolution.

```rust
// src/app.rs (new types)

pub struct ReferenceLayer {
    /// Pre-processed background colors at canvas resolution.
    /// Indexed [y][x]. Each cell is the dimmed reference color.
    pub colors: Vec<Vec<Option<Rgb>>>,
    /// Original image path (for project file persistence)
    pub image_path: String,
    /// Brightness level: 0=dim (25%), 1=medium (50%), 2=bright (75%)
    pub brightness: u8,
    /// Whether reference is currently visible
    pub visible: bool,
}
```

New fields in `App`:

```rust
pub reference_layer: Option<ReferenceLayer>,
```

#### 3.2.2 Image Processing

Reuses existing `import.rs` image loading (the `image` crate is already a dependency). The reference layer does NOT use `import_image()` directly because that function produces `Cell` data with characters. Instead, we extract just the color data.

```rust
// src/app.rs (new method)

impl App {
    pub fn load_reference(&mut self, path: &std::path::Path) -> Result<(), String> {
        let img = image::open(path)
            .map_err(|e| format!("Failed to load reference: {}", e))?;
        let img = img.resize_exact(
            self.canvas.width as u32,
            self.canvas.height as u32,
            image::imageops::FilterType::Lanczos3,
        );
        let mut colors = Vec::with_capacity(self.canvas.height);
        for y in 0..self.canvas.height {
            let mut row = Vec::with_capacity(self.canvas.width);
            for x in 0..self.canvas.width {
                let pixel = img.get_pixel(x as u32, y as u32);
                let [r, g, b, a] = pixel.0;
                if a < 128 {
                    row.push(None); // Transparent pixel
                } else {
                    row.push(Some(Rgb::new(r, g, b)));
                }
            }
            colors.push(row);
        }
        self.reference_layer = Some(ReferenceLayer {
            colors,
            image_path: path.to_string_lossy().to_string(),
            brightness: 0, // Start dim
            visible: true,
        });
        Ok(())
    }
}
```

#### 3.2.3 Brightness Dimming

Applied during rendering, not stored. The three levels scale RGB values:

| Level | Label | Scale Factor |
|-------|-------|-------------|
| 0 | Dim | 25% (r/4, g/4, b/4) |
| 1 | Medium | 50% (r/2, g/2, b/2) |
| 2 | Bright | 75% (3r/4, 3g/4, 3b/4) |

```rust
fn dim_color(color: &Rgb, brightness: u8) -> Rgb {
    let scale = match brightness {
        0 => 4,  // divide by 4 = 25%
        1 => 2,  // divide by 2 = 50%
        _ => 4,  // 3/4 = 75% — use (r*3)/4
    };
    if brightness == 2 {
        Rgb::new((color.r as u16 * 3 / 4) as u8,
                 (color.g as u16 * 3 / 4) as u8,
                 (color.b as u16 * 3 / 4) as u8)
    } else {
        Rgb::new(color.r / scale, color.g / scale, color.b / scale)
    }
}
```

#### 3.2.4 Canvas Rendering Integration

In `ui/editor.rs`, the `resolve_half_block_for_display` function changes to check the reference layer when a cell is empty/transparent:

```rust
// Current: empty cell → grid background
// New: empty cell → reference color (if visible) → grid background (fallback)

fn grid_or_reference_bg(
    x: usize, y: usize, show_grid: bool, theme: &Theme,
    reference: Option<&ReferenceLayer>,
) -> Color {
    if let Some(ref_layer) = reference {
        if ref_layer.visible {
            if let Some(Some(ref_color)) = ref_layer.colors.get(y).and_then(|row| row.get(x)) {
                let dimmed = dim_color(ref_color, ref_layer.brightness);
                return dimmed.to_ratatui(); // Uses Color::Indexed — safe
            }
        }
    }
    // Fallback: existing grid background
    grid_bg(x, y, show_grid, theme)
}
```

This modifies `resolve_half_block_for_display` and the zoom-4 half-block renderer to pass the reference layer. The reference only shows through transparent cells — opaque cells occlude it completely.

#### 3.2.5 Project File v6

The `Project` struct gains an optional field:

```rust
#[derive(Serialize, Deserialize)]
pub struct Project {
    pub version: u32,
    pub name: String,
    pub created_at: String,
    pub modified_at: String,
    pub color: Rgb,
    pub symmetry: SymmetryMode,
    pub canvas: Canvas,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub reference_image: Option<String>,  // Path to reference image
}
```

Version handling:
- `save_to_file`: If `reference_image.is_some()`, set version to 6. Otherwise, keep 5.
- `load_from_file`: Accept versions up to 6. The `#[serde(default)]` on `reference_image` means v5 files load with `None`.
- The reference `colors` grid is NOT serialized — it's rebuilt from the image path on project load.

#### 3.2.6 CLI Command

```rust
Command::Reference {
    file: String,           // .kaku project path
    image: Option<String>,  // image path (None when --clear)
    clear: bool,            // --clear flag
}
```

Flow:
1. Load project
2. If `--clear`: set `project.reference_image = None`, save, done
3. If image provided: validate image exists, set `project.reference_image = Some(path)`, save
4. Output JSON: `{"reference": "photo.png", "file": "art.kaku"}` or `{"reference": null, "file": "art.kaku"}`

#### 3.2.7 TUI Integration

On project load in `app.rs`, if `project.reference_image.is_some()`, call `load_reference()` to pre-process the image into the color grid.

Command palette adds two new commands:
- "Toggle Reference" — toggles `reference_layer.visible`
- "Reference Brightness" — cycles brightness 0→1→2→0

---

### 3.3 Batch Draw (FR-3)

#### 3.3.1 JSON Schema

```rust
// src/cli/batch.rs (new file)

use serde::Deserialize;

#[derive(Deserialize)]
pub struct BatchFile {
    pub operations: Vec<BatchOp>,
}

#[derive(Deserialize)]
#[serde(tag = "op")]
pub enum BatchOp {
    #[serde(rename = "draw")]
    Draw {
        tool: String,
        #[serde(default)]
        x: Option<usize>,
        #[serde(default)]
        y: Option<usize>,
        #[serde(default)]
        x1: Option<usize>,
        #[serde(default)]
        y1: Option<usize>,
        #[serde(default)]
        x2: Option<usize>,
        #[serde(default)]
        y2: Option<usize>,
        #[serde(default)]
        ch: Option<char>,
        #[serde(default)]
        fg: Option<String>,
        #[serde(default)]
        bg: Option<String>,
        #[serde(default)]
        filled: Option<bool>,
    },
    #[serde(rename = "set_cell")]
    SetCell {
        x: usize,
        y: usize,
        #[serde(default)]
        ch: Option<char>,
        #[serde(default)]
        fg: Option<String>,
        #[serde(default)]
        bg: Option<String>,
    },
    #[serde(rename = "clear")]
    Clear {
        #[serde(default)]
        region: Option<[usize; 4]>,
    },
    #[serde(rename = "resize")]
    Resize {
        width: usize,
        height: usize,
    },
}
```

The `#[serde(tag = "op")]` attribute makes serde parse `{"op": "draw", ...}` into `BatchOp::Draw { ... }` — the `op` field is the discriminant.

#### 3.3.2 Executor

```rust
pub fn run_batch(file: &str, commands_path: &str, dry_run: bool) -> io::Result<()> {
    // 1. Load project (single load)
    let path = Path::new(file);
    let mut project = load_project(file);

    // 2. Parse JSON
    let json_str = std::fs::read_to_string(commands_path)
        .unwrap_or_else(|e| cli_error(&format!("Cannot read '{}': {}", commands_path, e)));
    let batch: BatchFile = serde_json::from_str(&json_str)
        .unwrap_or_else(|e| cli_error(&format!("Invalid batch JSON: {}", e)));

    if dry_run {
        // Validate only — report op count and exit
        println!("{}", serde_json::json!({
            "dry_run": true,
            "operations": batch.operations.len(),
            "valid": true,
        }));
        return Ok(());
    }

    // 3. Execute operations in order
    let mut cells_modified: usize = 0;
    let mut errors: usize = 0;
    let mut error_details: Vec<serde_json::Value> = Vec::new();

    for (i, op) in batch.operations.iter().enumerate() {
        match execute_op(&mut project, op) {
            Ok(modified) => cells_modified += modified,
            Err(msg) => {
                errors += 1;
                error_details.push(serde_json::json!({
                    "index": i,
                    "error": msg,
                }));
            }
        }
    }

    // 4. Atomic save
    atomic_save(&mut project, path)?;

    // 5. Report
    let mut result = serde_json::json!({
        "operations": batch.operations.len(),
        "cells_modified": cells_modified,
        "errors": errors,
        "file": file,
    });
    if !error_details.is_empty() {
        result["error_details"] = serde_json::json!(error_details);
    }
    println!("{}", serde_json::to_string(&result).unwrap());
    Ok(())
}
```

#### 3.3.3 Operation Executor

Each operation maps directly to existing `tools::*` functions:

```rust
fn execute_op(project: &mut Project, op: &BatchOp) -> Result<usize, String> {
    match op {
        BatchOp::Draw { tool, x, y, x1, y1, x2, y2, ch, fg, bg, filled } => {
            let fg_rgb = parse_optional_color(fg)?;
            let bg_rgb = parse_optional_color(bg)?;
            let character = ch.unwrap_or(blocks::FULL);

            let mutations = match tool.as_str() {
                "pencil" => {
                    let (x, y) = require_xy(x, y)?;
                    tools::pencil(&project.canvas, x, y, character, fg_rgb, bg_rgb)
                }
                "eraser" => {
                    let (x, y) = require_xy(x, y)?;
                    tools::eraser(&project.canvas, x, y)
                }
                "line" => {
                    let (x1, y1, x2, y2) = require_rect_coords(x1, y1, x2, y2)?;
                    tools::line(&project.canvas, x1, y1, x2, y2, character, fg_rgb, bg_rgb)
                }
                "rect" => {
                    let (x1, y1, x2, y2) = require_rect_coords(x1, y1, x2, y2)?;
                    tools::rectangle(&project.canvas, x1, y1, x2, y2,
                                     character, fg_rgb, bg_rgb, filled.unwrap_or(false))
                }
                "fill" => {
                    let (x, y) = require_xy(x, y)?;
                    tools::flood_fill(&project.canvas, x, y, character, fg_rgb, bg_rgb)
                }
                other => return Err(format!("Unknown tool: '{}'", other)),
            };

            // Apply mutations directly to canvas
            for m in &mutations {
                project.canvas.set(m.x, m.y, m.new);
            }
            Ok(mutations.len())
        }
        BatchOp::SetCell { x, y, ch, fg, bg } => {
            let fg_rgb = parse_optional_color(fg)?;
            let bg_rgb = parse_optional_color(bg)?;
            let character = ch.unwrap_or(blocks::FULL);
            project.canvas.set(*x, *y, Cell { ch: character, fg: fg_rgb, bg: bg_rgb });
            Ok(1)
        }
        BatchOp::Clear { region } => {
            match region {
                Some([x1, y1, x2, y2]) => {
                    let mut count = 0;
                    for y in *y1..=*y2 {
                        for x in *x1..=*x2 {
                            project.canvas.set(x, y, Cell::default());
                            count += 1;
                        }
                    }
                    Ok(count)
                }
                None => {
                    let count = project.canvas.width * project.canvas.height;
                    project.canvas.clear();
                    Ok(count)
                }
            }
        }
        BatchOp::Resize { width, height } => {
            project.canvas.resize(*width, *height);
            Ok(0) // resize doesn't "modify cells" in the mutation sense
        }
    }
}
```

#### 3.3.4 CLI Command

```rust
Command::Batch {
    file: String,            // .kaku project path
    commands: String,        // path to JSON operations file
    dry_run: bool,           // --dry-run flag
}
```

Added to `Command` enum and routed in `cli::run()`.

#### 3.3.5 Helper Functions

```rust
fn parse_optional_color(hex: &Option<String>) -> Result<Option<Rgb>, String> {
    match hex {
        None => Ok(None),
        Some(s) => parse_hex_color(s)
            .map(Some)
            .ok_or_else(|| format!("Invalid color: '{}'", s)),
    }
}

fn require_xy(x: &Option<usize>, y: &Option<usize>) -> Result<(usize, usize), String> {
    match (x, y) {
        (Some(x), Some(y)) => Ok((*x, *y)),
        _ => Err("Missing required x,y coordinates".to_string()),
    }
}

fn require_rect_coords(
    x1: &Option<usize>, y1: &Option<usize>,
    x2: &Option<usize>, y2: &Option<usize>,
) -> Result<(usize, usize, usize, usize), String> {
    match (x1, y1, x2, y2) {
        (Some(x1), Some(y1), Some(x2), Some(y2)) => Ok((*x1, *y1, *x2, *y2)),
        _ => Err("Missing required x1,y1,x2,y2 coordinates".to_string()),
    }
}
```

---

## 4. Data Architecture

### Project File Format v6

Only change: optional `reference_image` field.

```json
{
  "version": 6,
  "name": "my-art",
  "created_at": "2026-02-28T12:00:00Z",
  "modified_at": "2026-02-28T12:30:00Z",
  "color": {"r": 255, "g": 255, "b": 255},
  "symmetry": "Off",
  "canvas": { ... },
  "reference_image": "reference.png"
}
```

- `reference_image`: Optional. Path to image file, stored relative to the project file's directory.
- Omitted entirely when `None` (via `skip_serializing_if`)
- v5 files load normally — `#[serde(default)]` handles missing field
- Version set to 6 only when `reference_image.is_some()`; otherwise stays 5
- `load_from_file` accepts versions up to 6

### No Other Data Changes

- Canvas: `Vec<Vec<Cell>>` — unchanged
- Oplog: JSON lines — unchanged
- Palette: `.palette` JSON — unchanged

---

## 5. File Changes Summary

| File | Change Type | Description |
|------|-------------|-------------|
| `src/app.rs` | **Modified** | Add `AppMode::CommandPalette`, `PaletteCommand`, `ReferenceLayer`, new state fields, `load_reference()` |
| `src/input.rs` | **Modified** | Add `handle_command_palette()`, modify Spacebar dispatch, add palette mode to dispatch table |
| `src/ui/mod.rs` | **Modified** | Add `render_command_palette()`, add `CommandPalette` to overlay match |
| `src/ui/editor.rs` | **Modified** | Pass reference layer to rendering, show reference behind transparent cells |
| `src/project.rs` | **Modified** | Add `reference_image: Option<String>`, bump version logic, accept v6 |
| `src/cli/mod.rs` | **Modified** | Add `Command::Batch` and `Command::Reference`, route to handlers |
| `src/cli/batch.rs` | **New** | Batch JSON parser and executor |

**Total**: 1 new file, 6 modified files.

**Unchanged**: `lib.rs`, `canvas.rs`, `cell.rs`, `export.rs`, `history.rs`, `import.rs`, `oplog.rs`, `palette.rs`, `symmetry.rs`, `theme.rs`, `tools.rs`, `ui/toolbar.rs`, `ui/palette.rs`, `ui/statusbar.rs`, all existing `cli/` submodules.

---

## 6. Testing Strategy

### Existing Tests (285+)

All run unchanged. No modifications to library modules.

### New Tests

| Test | Location | What It Validates |
|------|----------|------------------|
| Fuzzy matching | `app.rs` or `input.rs` tests | "sav" matches "Save", "sym h" matches "Symmetry Horizontal", empty matches all |
| Command registry completeness | `app.rs` tests | Every AppMode/ToolKind/SymmetryMode reachable via a command |
| Command palette open/close | `input.rs` tests | Spacebar opens when `!canvas_cursor_active`, Esc closes |
| Project v6 roundtrip | `project.rs` tests | Save with reference → load → reference path preserved |
| Project v5 backward compat | `project.rs` tests | v5 file loads with `reference_image == None` |
| Reference brightness dimming | `app.rs` tests | dim_color at each level produces expected values |
| Batch JSON parsing | `cli/batch.rs` tests | Valid JSON deserializes correctly, invalid JSON produces error |
| Batch pencil operation | `cli/batch.rs` tests | Single pencil op modifies expected cell |
| Batch rect operation | `cli/batch.rs` tests | Rect op produces expected cell mutations |
| Batch fill operation | `cli/batch.rs` tests | Fill op floods expected region |
| Batch set_cell | `cli/batch.rs` tests | Direct cell set with ch/fg/bg |
| Batch clear (region) | `cli/batch.rs` tests | Region clear resets expected cells |
| Batch clear (full) | `cli/batch.rs` tests | Full clear resets all cells |
| Batch resize | `cli/batch.rs` tests | Canvas dimensions change |
| Batch error handling | `cli/batch.rs` tests | Invalid tool name skipped, error counted, rest execute |
| Batch dry-run | `cli/batch.rs` tests | No mutations when dry_run=true |
| Batch empty file | `cli/batch.rs` tests | 0 operations, 0 cells modified |
| Batch multi-op ordering | `cli/batch.rs` tests | Later ops see state from earlier ops |

**Expected new test count**: ~18-20 tests. Total: 305+.

---

## 7. Implementation Order

```
Sprint 1: Command Palette + Batch Draw (CLI-focused)
  1. Command registry (PaletteCommand array, ~31 entries)
  2. Fuzzy matching function + tests
  3. AppMode::CommandPalette + state fields
  4. Input handler: Spacebar dispatch, palette key handling
  5. UI renderer: render_command_palette()
  6. cli/batch.rs: BatchFile/BatchOp types + serde
  7. Batch executor: execute_op() using tools::*
  8. Command::Batch CLI wiring + --dry-run
  9. Batch tests (JSON parsing, execution, errors)

Sprint 2: Reference Layer + Polish
  1. Project v6: add reference_image field + version logic
  2. Project v6 tests (roundtrip, backward compat)
  3. ReferenceLayer type + load_reference() method
  4. Reference CLI command (set/clear)
  5. editor.rs: reference rendering behind transparent cells
  6. Brightness dimming + toggle via command palette
  7. Reference layer tests
  8. Integration testing: all features end-to-end
```

Sprint 1 is larger but more parallelizable (palette and batch are independent). Sprint 2 builds on sprint 1's command palette to add reference toggle/brightness commands.

---

## 8. Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Spacebar conflict with canvas cursor | Context check: `if app.canvas_cursor_active` retains draw behavior, else opens palette. Exact same pattern as S-key dual behavior (line 410-419 of input.rs). |
| Reference layer performance | Pre-process image to `Vec<Vec<Option<Rgb>>>` on load. Rendering is a simple lookup per cell — no image processing per frame. Cache invalidated only on zoom change or reference reload. |
| Batch JSON parsing errors | serde handles malformed JSON with clear error messages. Per-operation errors are caught individually and don't halt the batch. |
| Project v6 backward compatibility | `#[serde(skip_serializing_if = "Option::is_none")]` + `#[serde(default)]` — standard serde pattern. Version bump is conditional. |
| Reference image path persistence | Store relative to project file directory. On load, resolve against project parent dir. If file not found, set reference to None with a warning — don't crash. |
| Color rendering | All reference colors go through `Rgb::to_ratatui()` which uses `Color::Indexed(nearest_256())`. No risk of `Color::Rgb()` leaking. |

---

## 9. Dependencies

No new crate dependencies. The `image` crate (already used by `import.rs`) handles reference image loading. `serde` and `serde_json` (already used everywhere) handle batch JSON parsing. `clap` (already used for CLI) handles new subcommands.

---

## 10. Future Considerations

This architecture enables:
- **Command palette history**: Track recently used commands, show at top of list
- **Batch stdin**: Replace file path with `-` to read from stdin (pipe-friendly)
- **MCP server**: The batch executor's `execute_op()` function is the natural entry point for a Model Context Protocol server — it's already JSON-in, state-mutation-out
- **Multiple reference images**: `ReferenceLayer` could become `Vec<ReferenceLayer>` with per-layer visibility
- **Keyboard shortcut remapping**: The command registry provides the indirection layer needed for customizable bindings
