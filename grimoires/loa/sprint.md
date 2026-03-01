# Sprint Plan: Creative Power Tools — Command Palette, Reference Layer, Batch Draw

> **Cycle**: 018
> **Created**: 2026-02-28
> **PRD**: grimoires/loa/prd.md
> **SDD**: grimoires/loa/sdd.md

---

## Sprint Overview

| Item | Value |
|------|-------|
| Total sprints | 2 |
| Sprint 1 | Command Palette + Batch Draw |
| Sprint 2 | Reference Layer + Integration Polish |
| Estimated new tests | 18-20 |
| Target total tests | 305+ |
| New files | 1 (`src/cli/batch.rs`) |
| Modified files | 6 |

---

## Sprint 1: Command Palette + Batch Draw

**Goal**: Deliver the two independent features — the TUI command palette (human discoverability) and the CLI batch draw command (agent throughput).

### Task 1.1: Command Registry

**Description**: Create the `PaletteCommand` struct and static `COMMANDS` array in `app.rs` with all ~31 editor commands mapped to function pointers.

**Files**: `src/app.rs`

**Acceptance Criteria**:
- [ ] `PaletteCommand` struct with `name`, `category`, `shortcut`, `action: fn(&mut App)` fields
- [ ] Static `COMMANDS` array with all commands across 9 categories (Tools, Canvas, File, Edit, View, Character, Color, Symmetry, Help)
- [ ] Each command's `action` correctly invokes the corresponding App method
- [ ] Test: every ToolKind variant is reachable via a command
- [ ] Test: every SymmetryMode variant is reachable via a command

### Task 1.2: Fuzzy Matching

**Description**: Implement the `fuzzy_match(query, target) -> bool` function for filtering commands. Subsequence matching with space-skip.

**Files**: `src/app.rs`

**Acceptance Criteria**:
- [ ] `fuzzy_match("sav", "Save")` returns true
- [ ] `fuzzy_match("sav", "Save As")` returns true
- [ ] `fuzzy_match("sym h", "Symmetry Horizontal")` returns true
- [ ] `fuzzy_match("xyz", "Save")` returns false
- [ ] `fuzzy_match("", "anything")` returns true (empty matches all)
- [ ] Case-insensitive matching
- [ ] 6+ unit tests for fuzzy matching

### Task 1.3: Command Palette Mode + State

**Description**: Add `AppMode::CommandPalette` variant and palette state fields to `App`. Wire up initialization in the `App::new()` constructor.

**Files**: `src/app.rs`

**Acceptance Criteria**:
- [ ] `AppMode::CommandPalette` variant added to enum
- [ ] New fields: `palette_query: String`, `palette_filtered: Vec<usize>`, `palette_selected: usize`
- [ ] Fields initialized in `App::new()`
- [ ] Compiles with no warnings

### Task 1.4: Command Palette Input Handling

**Description**: Modify the Spacebar dispatch in `handle_key()` and add `handle_command_palette()` to the mode dispatch table in `handle_event()`.

**Files**: `src/input.rs`

**Acceptance Criteria**:
- [ ] Spacebar opens palette when `!canvas_cursor_active` (existing draw behavior preserved when active)
- [ ] `AppMode::CommandPalette` added to the mode dispatch table (before the `_ => {}` fallthrough)
- [ ] `handle_command_palette()` handles: printable chars (filter), Backspace (delete), Up/Down (navigate), Enter (execute), Esc (dismiss)
- [ ] On Enter: selected command's `action` is called, mode returns to Normal
- [ ] On Esc: mode returns to Normal, no action
- [ ] `palette_selected` wraps at list bounds

### Task 1.5: Command Palette Renderer

**Description**: Create `render_command_palette()` in `ui/mod.rs` and add it to the overlay match in `render()`.

**Files**: `src/ui/mod.rs`

**Acceptance Criteria**:
- [ ] Centered overlay in top-third of screen, ~50 chars wide
- [ ] Text input line with `"> "` prefix showing current query
- [ ] Filtered command list with selected item highlighted (`theme.highlight`)
- [ ] Each command shows `name` left-aligned, `shortcut` right-aligned in `theme.dim`
- [ ] Max ~10 visible commands with scroll if more results
- [ ] Theme-aware via `app.theme()`
- [ ] `AppMode::CommandPalette` case added to overlay match in `render()`
- [ ] Clear widget behind overlay (prevents bleed-through)

### Task 1.6: Batch JSON Types

**Description**: Create `src/cli/batch.rs` with `BatchFile` and `BatchOp` serde types. Register the module in `cli/mod.rs`.

**Files**: `src/cli/batch.rs` (new), `src/cli/mod.rs`

**Acceptance Criteria**:
- [ ] `BatchFile` with `operations: Vec<BatchOp>`
- [ ] `BatchOp` enum with `#[serde(tag = "op")]`: Draw, SetCell, Clear, Resize
- [ ] Draw variant: tool, x, y, x1, y1, x2, y2, ch, fg, bg, filled (all optional except tool)
- [ ] SetCell variant: x, y (required), ch, fg, bg (optional)
- [ ] Clear variant: region (optional `[usize; 4]`)
- [ ] Resize variant: width, height (required)
- [ ] Test: valid JSON deserializes to correct BatchOp variants
- [ ] Test: malformed JSON produces clear serde error

### Task 1.7: Batch Executor

**Description**: Implement `execute_op()` and `run_batch()` functions in `cli/batch.rs`. Each operation maps to existing `tools::*` functions.

**Files**: `src/cli/batch.rs`

**Acceptance Criteria**:
- [ ] `execute_op()` dispatches draw/pencil to `tools::pencil()`, draw/line to `tools::line()`, etc.
- [ ] `execute_op()` applies mutations directly to `project.canvas`
- [ ] Per-operation errors caught and reported (don't halt batch)
- [ ] `run_batch()` loads project once, executes all ops in order, saves atomically
- [ ] `--dry-run` validates JSON and reports op count without executing
- [ ] JSON output: `{"operations": N, "cells_modified": M, "errors": E, "file": "..."}`
- [ ] Error details included when errors > 0
- [ ] Helper functions: `parse_optional_color()`, `require_xy()`, `require_rect_coords()`

### Task 1.8: Batch CLI Wiring

**Description**: Add `Command::Batch` to the CLI enum and route it in `cli::run()`.

**Files**: `src/cli/mod.rs`

**Acceptance Criteria**:
- [ ] `Command::Batch { file, commands, dry_run }` variant with clap attributes
- [ ] `--commands <path>` required argument for JSON file path
- [ ] `--dry-run` optional flag
- [ ] Routed to `batch::run_batch()` in `cli::run()`
- [ ] `pub mod batch;` added to `cli/mod.rs`

### Task 1.9: Batch Tests

**Description**: Comprehensive unit tests for batch operations.

**Files**: `src/cli/batch.rs` (test module)

**Acceptance Criteria**:
- [ ] Test: pencil op modifies expected cell
- [ ] Test: rect op draws expected outline
- [ ] Test: fill op floods connected region
- [ ] Test: line op draws between two points
- [ ] Test: set_cell sets ch/fg/bg directly
- [ ] Test: clear region resets specified cells
- [ ] Test: clear full resets all cells
- [ ] Test: resize changes canvas dimensions
- [ ] Test: unknown tool name produces error, doesn't halt
- [ ] Test: multi-op ordering — later ops see state from earlier ops
- [ ] Test: empty operations array → 0 cells modified
- [ ] Test: dry_run does not modify canvas
- [ ] All 285+ existing tests still pass

---

## Sprint 2: Reference Layer + Integration Polish

**Goal**: Deliver the reference layer (TUI + CLI, project v6) and ensure all three features integrate cleanly.

### Task 2.1: Project File v6

**Description**: Add `reference_image: Option<String>` to the `Project` struct with serde attributes for backward compatibility. Update version handling.

**Files**: `src/project.rs`

**Acceptance Criteria**:
- [x] `reference_image: Option<String>` field with `#[serde(skip_serializing_if = "Option::is_none")]` and `#[serde(default)]`
- [x] `save_to_file`: sets version to 6 when `reference_image.is_some()`, keeps 5 otherwise
- [x] `load_from_file`: accepts versions up to 6
- [x] `Project::new()` initializes `reference_image: None`
- [x] Test: v6 project roundtrip (save with reference → load → path preserved)
- [x] Test: v5 project loads with `reference_image == None`
- [x] Test: v5 project without reference saves as v5 (no version bump)

### Task 2.2: Reference Layer Type + Image Loading

**Description**: Add `ReferenceLayer` struct and `load_reference()` method to `App`. Pre-processes image into `Vec<Vec<Option<Rgb>>>` color grid.

**Files**: `src/app.rs`

**Acceptance Criteria**:
- [x] `ReferenceLayer` struct: `colors`, `image_path`, `brightness`, `visible`
- [x] `reference_layer: Option<ReferenceLayer>` field in App, initialized to None
- [x] `load_reference()` opens image, resizes to canvas dimensions, extracts RGB colors
- [x] Transparent pixels (alpha < 128) stored as None
- [x] On project load: if `reference_image.is_some()`, resolve path relative to project dir and call `load_reference()`
- [x] If image file missing on load: set reference to None, show warning (don't crash)
- [x] Test: dim_color at brightness 0/1/2 produces expected values

### Task 2.3: Reference CLI Command

**Description**: Add `Command::Reference` to CLI for setting and clearing reference images.

**Files**: `src/cli/mod.rs`

**Acceptance Criteria**:
- [x] `Command::Reference { file, image, clear }` with clap attributes
- [x] `kakukuma reference <file> <image>` sets reference_image and saves
- [x] `kakukuma reference <file> --clear` removes reference_image and saves
- [x] Image path stored relative to project file directory
- [x] Validates image file exists before setting
- [x] JSON output: `{"reference": "photo.png", "file": "art.kaku"}` or `{"reference": null, ...}`
- [x] Atomic save via existing `atomic_save()` pattern

### Task 2.4: Reference Rendering in Editor

**Description**: Modify `ui/editor.rs` to show reference layer colors behind transparent canvas cells. Works at all zoom levels.

**Files**: `src/ui/editor.rs`

**Acceptance Criteria**:
- [x] New `grid_or_reference_bg()` function replaces `grid_bg()` for empty cells
- [x] Reference colors show through transparent cells at dimmed brightness
- [x] Opaque cells fully occlude reference
- [x] `resolve_half_block_for_display` passes reference layer
- [x] Works at zoom 1x, 2x, and 4x (half-block zoom)
- [x] All colors go through `Rgb::to_ratatui()` — no `Color::Rgb()` leaks
- [x] Render signature changes: `editor::render()` accepts `Option<&ReferenceLayer>`
- [x] `ui/mod.rs` passes `app.reference_layer.as_ref()` to editor

### Task 2.5: Reference Toggle + Brightness via Command Palette

**Description**: Add "Toggle Reference" and "Reference Brightness" commands to the palette registry. Wire brightness cycling.

**Files**: `src/app.rs`

**Acceptance Criteria**:
- [x] "Toggle Reference" command in COMMANDS array — toggles `reference_layer.visible`
- [x] "Reference Brightness" command — cycles brightness 0→1→2→0
- [x] Both commands gracefully no-op when no reference loaded
- [x] Status message shows current brightness level after cycling

### Task 2.6: Integration Testing + Regression Check

**Description**: End-to-end validation of all three features working together. Ensure no regressions.

**Files**: Various (test execution only)

**Acceptance Criteria**:
- [x] All 285+ existing tests pass
- [x] New tests bring total to 305+
- [x] `cargo clippy` — no new warnings
- [x] Command palette opens/closes correctly in TUI
- [x] Batch command executes multi-op JSON file
- [x] Reference layer renders behind canvas in TUI (manual visual check)
- [x] Project v5 loads without issues
- [x] Project v6 roundtrip with reference image

---

## Risk Assessment

| Risk | Sprint | Mitigation |
|------|--------|------------|
| `tools::flood_fill` signature differs from SDD assumption | Sprint 1 | Verify during Task 1.7, adapt batch executor if needed |
| Spacebar conflicts with canvas cursor | Sprint 1 | Context check in Task 1.4, same pattern as S-key dual behavior |
| Reference image path breaks on different CWD | Sprint 2 | Store relative to project dir, resolve on load (Task 2.3) |
| Reference rendering performance | Sprint 2 | Pre-process to color grid on load, not per-frame (Task 2.2) |
