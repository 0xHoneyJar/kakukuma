# PRD: Kakukuma Agent Mode — Programmatic Drawing Interface

> **Version**: 1.0
> **Status**: Draft
> **Author**: Claude (from `/plan-and-analyze` discovery)
> **Date**: 2026-02-14

## 1. Problem Statement

Kakukuma is a capable terminal ANSI art editor, but it is exclusively interactive — all drawing operations require a human at a keyboard using the TUI. There is no way for AI agents to create, manipulate, or inspect art programmatically. This blocks a compelling use case: training and evaluating agent spatial reasoning through pixel-art creation.

> Sources: Codebase analysis (src/input.rs — all operations are event-driven via crossterm), user interview Phase 1

## 2. Vision

Make Kakukuma a dual-interface art tool where both humans and AI agents can create ANSI art with equal capability. The TUI remains the primary human interface. A new CLI subcommand interface provides agents with low-friction, stateless drawing operations on `.kaku` files, enabling a tight **draw → preview → evaluate → iterate** feedback loop for spatial reasoning development.

## 3. Goals & Success Criteria

| Goal | Success Metric |
|------|----------------|
| **Agent can create recognizable art** | An agent produces a recognizable image (face, house, geometric pattern) using only CLI commands |
| **Sub-second feedback loop** | Draw + preview round-trip completes in < 1 second for a 48x32 canvas |
| **Full feature parity** | 100% of TUI drawing features accessible via CLI — no capability gap |
| **Rich inspection** | Agent can query canvas state as structured JSON, view diffs, and get statistics |
| **Quality** | All CLI operations have unit tests, integration tests, and documented behavior |

## 4. User Personas

### Persona 1: Human Artist (Existing)
- Uses the TUI interactively
- Draws with mouse and keyboard
- Saves/loads .kaku project files
- **No change to their workflow** — TUI remains unchanged

### Persona 2: AI Agent (New)
- Executes shell commands via Bash tool
- Cannot interact with TUI (no keyboard/mouse simulation)
- Needs structured output (JSON) for programmatic analysis
- Iterates rapidly: draw → preview → evaluate → draw more
- May have vision capabilities (can interpret ANSI art in terminal)
- Goal: develop and demonstrate spatial reasoning through art creation

### Persona 3: Agent Trainer (New)
- Human who sets up agent tasks and evaluates results
- Uses TUI to view agent-created .kaku files
- Uses `kakukuma diff` to compare agent output against reference art
- Uses `kakukuma stats` to quantify agent performance

## 5. Functional Requirements

### 5.1 CLI Subcommand Architecture

**Single binary, subcommand pattern** (recommended architecture):

```
kakukuma                     → Launch TUI (default, existing behavior)
kakukuma new [OPTIONS] FILE  → Create new .kaku file
kakukuma draw TOOL FILE ARGS → Draw on canvas
kakukuma preview FILE        → Render canvas to stdout
kakukuma inspect FILE        → Query canvas cell data
kakukuma export FILE         → Export to ANSI/text/JSON
kakukuma diff FILE1 FILE2    → Compare two canvases
kakukuma stats FILE          → Canvas statistics
kakukuma undo FILE           → Undo last CLI operation
kakukuma redo FILE           → Redo last undone operation
kakukuma palette SUBCOMMAND  → Palette management
kakukuma history FILE        → Show operation log
```

When invoked with no subcommand (or with a `.kaku` file argument directly), the existing TUI launches. All new functionality lives behind explicit subcommands.

### 5.2 Canvas Creation

```bash
kakukuma new art.kaku                          # Default 48x32
kakukuma new --width 64 --height 48 art.kaku   # Custom size
kakukuma new --size 32x24 art.kaku             # Shorthand
```

**Behavior**:
- Creates a new `.kaku` file with an empty canvas
- Initializes an empty operation log (`.kaku.log`)
- Fails if file already exists (use `--force` to overwrite)
- Dimensions clamped to 8-128 (matching TUI behavior)

### 5.3 Drawing Commands

All drawing commands follow: `kakukuma draw <tool> <file> <coordinates> [options]`

#### 5.3.1 Pencil
```bash
kakukuma draw pencil art.kaku 5,5 --color "#FF6600"
kakukuma draw pencil art.kaku 5,5 --fg "#FF6600" --bg "#000000"
kakukuma draw pencil art.kaku 5,5 --color "#FF6600" --char "▀"
```

#### 5.3.2 Eraser
```bash
kakukuma draw eraser art.kaku 5,5
kakukuma draw eraser art.kaku 5,5 --region 5,5,10,10  # Erase area
```

#### 5.3.3 Line
```bash
kakukuma draw line art.kaku 0,0 15,15 --color "#00FF00"
```

#### 5.3.4 Rectangle
```bash
kakukuma draw rect art.kaku 2,2 10,8 --color "#0044FF"
kakukuma draw rect art.kaku 2,2 10,8 --color "#0044FF" --filled
```

#### 5.3.5 Fill (Flood Fill)
```bash
kakukuma draw fill art.kaku 12,12 --color "#FFFF00"
```

#### 5.3.6 Eyedropper
```bash
kakukuma draw eyedropper art.kaku 5,5
# Output: {"fg": "#FF6600", "bg": null, "char": "█"}
```

#### Common Drawing Options

| Flag | Description | Default |
|------|-------------|---------|
| `--color HEX` | Set foreground color | Current/white |
| `--fg HEX` | Set foreground explicitly | None |
| `--bg HEX` | Set background explicitly | None |
| `--char CHAR` | Block character to use | `█` (full block) |
| `--symmetry MODE` | Apply symmetry: `off`, `horizontal`, `vertical`, `quad` | `off` |
| `--no-log` | Skip operation log (no undo for this operation) | Log enabled |

### 5.4 Preview

```bash
kakukuma preview art.kaku                    # ANSI art to stdout
kakukuma preview art.kaku --format json      # JSON grid to stdout
kakukuma preview art.kaku --format ansi      # Explicit ANSI (default)
kakukuma preview art.kaku --region 0,0,15,15 # Preview subregion
kakukuma preview art.kaku --color-format 256 # Force 256-color output
kakukuma preview art.kaku --color-format 16  # Force 16-color output
```

**ANSI output**: Renders the canvas as colored terminal text using half-block characters (reuses existing `export::to_ansi()`).

**JSON output**:
```json
{
  "width": 48,
  "height": 32,
  "cells": [
    [{"x": 0, "y": 0, "fg": "#FF6600", "bg": null, "char": "█"}, ...],
    ...
  ],
  "non_empty_count": 42
}
```

### 5.5 Inspection

```bash
kakukuma inspect art.kaku 5,5
# Output: {"x": 5, "y": 5, "fg": "#FF6600", "bg": null, "char": "█"}

kakukuma inspect art.kaku --region 0,0,10,10
# Output: JSON array of non-empty cells in region

kakukuma inspect art.kaku --row 5
# Output: JSON array of cells in row 5

kakukuma inspect art.kaku --col 5
# Output: JSON array of cells in column 5
```

### 5.6 Diff

```bash
kakukuma diff art1.kaku art2.kaku
# Output: JSON list of changed cells

kakukuma diff art.kaku --before  # Compare current state vs before last operation
```

**Output format**:
```json
{
  "changes": [
    {"x": 5, "y": 5, "before": {"fg": "#FF0000", "char": "█"}, "after": {"fg": "#00FF00", "char": "█"}},
    ...
  ],
  "added": 3,
  "removed": 1,
  "modified": 7,
  "unchanged": 1525
}
```

### 5.7 Statistics

```bash
kakukuma stats art.kaku
```

**Output**:
```json
{
  "canvas": {"width": 48, "height": 32, "total_cells": 1536},
  "fill": {"empty": 1494, "filled": 42, "fill_percent": 2.7},
  "colors": {
    "unique_fg": 5,
    "unique_bg": 2,
    "distribution": [
      {"color": "#FF6600", "count": 15, "percent": 35.7},
      ...
    ]
  },
  "characters": {
    "unique": 3,
    "distribution": [
      {"char": "█", "count": 30, "percent": 71.4},
      {"char": "▀", "count": 8, "percent": 19.0},
      {"char": "▄", "count": 4, "percent": 9.5}
    ]
  },
  "bounding_box": {"min_x": 2, "min_y": 3, "max_x": 20, "max_y": 15},
  "symmetry_score": {
    "horizontal": 0.85,
    "vertical": 0.42
  }
}
```

### 5.8 Undo/Redo (Operation Log)

Each CLI drawing command appends a JSON entry to `<file>.log`:

```json
{"timestamp": "2026-02-14T10:30:00Z", "command": "draw pencil", "mutations": [{"x": 5, "y": 5, "old": {"fg": null, "bg": null, "char": " "}, "new": {"fg": "#FF6600", "bg": null, "char": "█"}}]}
```

```bash
kakukuma undo art.kaku           # Undo last operation
kakukuma undo art.kaku --count 3 # Undo last 3 operations
kakukuma redo art.kaku           # Redo last undone operation
kakukuma history art.kaku        # List operation log (summary)
kakukuma history art.kaku --full # List with full mutation details
```

**Constraints**:
- Maximum 256 operations in log (matching TUI history limit)
- Older operations are pruned on new writes
- `--no-log` flag on draw commands skips logging

### 5.9 Palette Management

```bash
kakukuma palette list                           # List available .palette files
kakukuma palette show PALETTE_NAME              # Show colors in palette
kakukuma palette create "My Palette" art.kaku   # Create palette from colors used in canvas
kakukuma palette export PALETTE_NAME --output palette.json
kakukuma palette add PALETTE_NAME "#FF6600"     # Add color to custom palette
```

### 5.10 Theme Access

```bash
kakukuma palette themes              # List available themes (warm, neon, dark)
kakukuma palette theme warm          # Show colors in warm theme
```

## 6. Technical Requirements

### 6.1 Architecture: Single Binary with Subcommands

**Recommended approach** (Rust best practice):

```
src/
├── main.rs          Entry point — routes to TUI or CLI
├── cli/
│   ├── mod.rs       clap argument parser & subcommand routing
│   ├── draw.rs      Draw subcommand handlers
│   ├── preview.rs   Preview/export handlers
│   ├── inspect.rs   Inspection handlers
│   ├── diff.rs      Diff handler
│   ├── stats.rs     Statistics handler
│   ├── history.rs   Undo/redo/history handlers
│   └── palette.rs   Palette management handlers
├── ops/
│   ├── mod.rs       Shared drawing operations (used by both TUI and CLI)
│   ├── draw.rs      Tool application logic (extracted from tools.rs + app.rs)
│   ├── canvas.rs    Canvas manipulation (wraps existing canvas.rs)
│   ├── symmetry.rs  Symmetry application (wraps existing symmetry.rs)
│   └── log.rs       Operation logging for CLI undo/redo
├── app.rs           TUI application state (existing, calls ops/)
├── canvas.rs        Canvas data structure (existing)
├── cell.rs          Cell/Color types (existing)
├── tools.rs         Tool implementations (existing, reused by ops/)
├── ...              Other existing modules unchanged
```

**Key principle**: Extract shared logic into `ops/` module. Both `app.rs` (TUI) and `cli/` call `ops/`. Existing modules (`tools.rs`, `canvas.rs`, `cell.rs`, etc.) remain unchanged — `ops/` composes them.

### 6.2 Dependencies (New)

| Crate | Purpose |
|-------|---------|
| `clap` (v4, derive) | CLI argument parsing with subcommands |
| `serde_json` | Already present — reused for JSON output |

No other new dependencies required. Existing crates (`serde`, `ratatui`, `crossterm`) remain for TUI.

### 6.3 Performance Requirements

| Operation | Target | Notes |
|-----------|--------|-------|
| `new` | < 50ms | File creation only |
| `draw` (single cell) | < 100ms | Load + mutate + save |
| `draw` (line/rect) | < 200ms | Multiple cell mutations |
| `draw` (fill) | < 500ms | Flood fill on large canvas |
| `preview --format ansi` | < 200ms | Reuses existing export |
| `preview --format json` | < 100ms | Serialization only |
| `inspect` | < 50ms | Load + read |
| `diff` | < 200ms | Load two files + compare |
| `stats` | < 200ms | Load + analyze |
| `undo` | < 200ms | Load + apply inverse + save |

### 6.4 Error Handling

All CLI commands exit with:
- **0**: Success
- **1**: User error (bad arguments, file not found, coordinates out of bounds)
- **2**: Internal error (file corruption, I/O failure)

Errors output to stderr as structured JSON when `--json-errors` flag is set, plain text otherwise:
```json
{"error": "coordinates_out_of_bounds", "message": "Position (150, 5) exceeds canvas dimensions (48x32)", "code": 1}
```

### 6.5 Non-Functional Requirements

- **No TUI dependency**: CLI mode must not initialize crossterm, ratatui, or alternate screen
- **Idempotent reads**: `preview`, `inspect`, `diff`, `stats` never modify the .kaku file
- **Atomic writes**: Drawing operations load → mutate → write atomically (temp file + rename)
- **Backward compatible**: CLI can read all .kaku versions (v1-v5), always writes v5

## 7. Scope

### 7.1 In Scope (This Cycle)

- CLI subcommand infrastructure (clap integration)
- All 6 drawing tools via CLI with full option parity
- Preview in ANSI and JSON formats
- Cell/region inspection
- Canvas diff between files
- Canvas statistics with symmetry scoring
- Operation log with undo/redo
- Palette listing, viewing, creation
- Theme color access
- Comprehensive test coverage
- Extraction of shared ops module

### 7.2 Out of Scope

| Feature | Reason |
|---------|--------|
| MCP Server | Future cycle — CLI is sufficient for initial agent integration |
| Web/WASM version | Future cycle — different platform target |
| Scripting language / DSL | CLI commands are sufficient; agents compose via Bash |
| Animation/frames | Separate feature track |
| Layers | Separate feature track |
| Live agent-TUI bridge | Future — agents watching TUI in real-time |

## 8. Risks & Mitigations

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| **File I/O overhead** per CLI command | Slow feedback loop | Medium | Profile early; consider memory-mapped files if needed |
| **Concurrent access** to same .kaku file | Data corruption | Low | Atomic writes (temp + rename); document single-writer constraint |
| **Operation log growth** | Disk usage | Low | 256-entry cap with pruning |
| **clap dependency size** | Binary bloat | Low | Use `clap` derive feature only; minimal features |
| **Breaking existing TUI behavior** | User regression | Medium | TUI code paths unchanged; new code in separate modules |
| **Half-block compositing complexity** in CLI | Incorrect rendering | Medium | Reuse existing `compose_cell()` and `resolve_half_block()` |

## 9. Open Questions

1. Should `kakukuma draw` support batch mode (multiple operations in one command via stdin)?
2. Should there be a `kakukuma watch` command that monitors a .kaku file and re-renders preview on change?
3. Should the symmetry score in `stats` use a specific algorithm (e.g., pixel-wise comparison of mirrored halves)?

## 10. Dependencies

- Rust toolchain (2021 edition, already in use)
- `clap` v4 with derive feature (new dependency)
- All other dependencies already present in Cargo.toml

## 11. Acceptance Criteria

1. `kakukuma` with no args launches TUI (unchanged behavior)
2. `kakukuma new --size 32x24 test.kaku` creates a valid .kaku file
3. `kakukuma draw pencil test.kaku 5,5 --color "#FF0000"` places a red cell at (5,5)
4. `kakukuma preview test.kaku` renders ANSI art to stdout
5. `kakukuma preview test.kaku --format json` outputs valid JSON with cell data
6. `kakukuma inspect test.kaku 5,5` returns the cell data as JSON
7. `kakukuma stats test.kaku` returns fill percentage, color distribution, bounding box
8. `kakukuma undo test.kaku` reverses the last drawing operation
9. `kakukuma diff a.kaku b.kaku` shows cell-level differences
10. All drawing tools (pencil, eraser, line, rect, fill, eyedropper) work via CLI
11. `--symmetry` flag applies mirrored drawing correctly
12. Operation log is created and maintained correctly
13. TUI opens and works identically to before (no regression)
14. All operations complete within performance targets
15. Comprehensive test coverage for all CLI paths
