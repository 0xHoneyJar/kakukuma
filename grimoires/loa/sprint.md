# Sprint Plan: Kakukuma Agent Mode

> **Cycle**: cycle-001 — Agent Mode: Programmatic Drawing Interface
> **Date**: 2026-02-14
> **PRD**: `grimoires/loa/prd.md`
> **SDD**: `grimoires/loa/sdd.md`
> **Sprints**: 3
> **Quality stance**: Quality over speed

---

## Sprint 1: Foundation + Core Drawing (MVP) ✅

**Goal**: An agent can create a canvas, draw with all 6 tools, and preview the result. This is the minimum for the draw-preview-iterate loop.

**Global ID**: sprint-1

### Task 1.1: Add clap dependency and restructure main.rs

**Description**: Add `clap` v4 (derive) to `Cargo.toml`. Restructure `main.rs` to parse CLI args first: if a subcommand is present, route to CLI path; otherwise launch TUI. The TUI path must remain byte-for-byte identical in behavior.

**Acceptance Criteria**:
- `clap = { version = "4", features = ["derive"] }` added to `Cargo.toml`
- `cargo build` succeeds with no warnings
- `cargo run` (no args) launches TUI exactly as before
- `cargo run -- myart.kaku` opens file in TUI (existing behavior preserved)
- `cargo run -- --help` shows help text with available subcommands
- All existing tests pass (`cargo test`)

**Estimated effort**: Small
**Dependencies**: None

---

### Task 1.2: Create oplog module for operation logging

**Description**: Create `src/oplog.rs` — the operation log that enables CLI undo/redo. JSON Lines format with a header line tracking the undo pointer. Supports append, read, undo (pointer move), redo (pointer move), and pruning at 256 entries. Make `project::now_iso8601()` `pub(crate)` for timestamp access.

**Acceptance Criteria**:
- `oplog::LogEntry`, `LogMutation`, `LogCell`, `LogHeader` structs defined with Serialize/Deserialize
- `oplog::log_path()` derives `.kaku.log` path from `.kaku` path
- `oplog::init_log()` creates empty log with `{"pointer":0,"total":0}` header
- `oplog::append()` adds entry, truncates undone entries, prunes to 256 max
- `oplog::read_log()` returns all entries (active + undone)
- `oplog::active_entries()` returns only entries up to pointer
- `oplog::pop_for_undo()` decrements pointer, returns undone entries
- `oplog::push_for_redo()` increments pointer, returns redone entries
- Unit tests: append, read, undo pointer movement, redo, pruning at 256, empty log handling, corrupt log graceful failure
- `project::now_iso8601()` changed to `pub(crate)`

**Estimated effort**: Medium
**Dependencies**: None

---

### Task 1.3: Create CLI scaffold with clap argument definitions

**Description**: Create `src/cli/mod.rs` with the full `Cli`, `Command`, `DrawTool`, `DrawOpts`, and all enum definitions. Implement `parse_coord()`, `parse_region()`, `parse_size()` parsers. Create stub `cli::run()` that routes to subcommand handlers. Create empty handler files: `draw.rs`, `preview.rs`, `inspect.rs`, `diff.rs`, `stats.rs`, `history_cmd.rs`, `palette_cmd.rs`.

**Acceptance Criteria**:
- `src/cli/mod.rs` compiles with all `Command` variants
- `parse_coord("5,5")` returns `Ok((5, 5))`
- `parse_coord("abc")` returns descriptive `Err`
- `parse_region("0,0,10,10")` returns `Ok((0, 0, 10, 10))`
- `parse_size("32x24")` returns `Ok((32, 24))`
- `cli::run()` dispatches to stub handlers (can print "not implemented" for now)
- `cargo run -- new --help` shows usage for the `new` subcommand
- `cargo run -- draw --help` shows available tools
- Unit tests for all parsers

**Estimated effort**: Medium
**Dependencies**: Task 1.1

---

### Task 1.4: Implement `new` command

**Description**: Implement the `new` subcommand that creates a `.kaku` project file with an empty canvas. Supports `--width`, `--height`, `--size`, and `--force` flags. Initializes the operation log. Outputs JSON summary to stdout.

**Acceptance Criteria**:
- `kakukuma new test.kaku` creates a valid `.kaku` file with 48x32 canvas
- `kakukuma new --width 64 --height 48 test.kaku` creates 64x48 canvas
- `kakukuma new --size 32x24 test.kaku` creates 32x24 canvas
- Dimensions clamped to 8-128 range
- Fails with exit code 1 if file already exists
- `--force` overwrites existing file
- Creates `test.kaku.log` operation log alongside
- Outputs `{"created":"test.kaku","width":48,"height":32}` to stdout
- Created file can be opened in TUI (`cargo run -- test.kaku`)
- Unit tests for dimension clamping, file-exists check, force overwrite

**Estimated effort**: Small
**Dependencies**: Task 1.2, Task 1.3

---

### Task 1.5: Implement `draw` command — all 6 tools

**Description**: Implement `src/cli/draw.rs` with handlers for pencil, eraser, line, rect, fill, and eyedropper. Each handler: loads project, parses colors/char, calls the existing `tools::` function, applies symmetry, applies mutations to canvas, logs to oplog (unless `--no-log`), saves atomically (temp + rename), outputs JSON result.

**Acceptance Criteria**:
- `kakukuma draw pencil test.kaku 5,5 --color "#FF0000"` places red cell at (5,5)
- `kakukuma draw pencil test.kaku 5,5 --fg "#FF0000" --bg "#0000FF"` sets both fg and bg
- `kakukuma draw pencil test.kaku 5,5 --color "#FF0000" --char "▀"` uses specified block char
- `kakukuma draw eraser test.kaku 5,5` clears cell to default
- `kakukuma draw line test.kaku 0,0 15,15 --color "#00FF00"` draws Bresenham line
- `kakukuma draw rect test.kaku 2,2 10,8 --color "#0044FF"` draws outline rectangle
- `kakukuma draw rect test.kaku 2,2 10,8 --color "#0044FF" --filled` draws filled rectangle
- `kakukuma draw fill test.kaku 12,12 --color "#FFFF00"` flood fills from point
- `kakukuma draw eyedropper test.kaku 5,5` outputs cell data as JSON (read-only, no file modification)
- `--symmetry horizontal` mirrors drawing across vertical center axis
- `--symmetry vertical` mirrors across horizontal center axis
- `--symmetry quad` mirrors both axes (4-way)
- `--no-log` skips operation log entry
- Outputs `{"ok":true,"cells_modified":N,"tool":"pencil","symmetry":"off"}` to stdout
- Invalid coordinates produce exit code 1 with descriptive error
- Invalid color format produces exit code 1 with descriptive error
- File write is atomic (temp file + rename)
- Operation is logged to `.kaku.log` by default
- Unit tests: color resolution logic, symmetry mapping, each tool invocation, bounds checking, atomic save

**Estimated effort**: Large
**Dependencies**: Task 1.4

---

### Task 1.6: Implement `preview` command

**Description**: Implement `src/cli/preview.rs` with ANSI and JSON output formats. ANSI mode reuses existing `export::to_ansi()`. JSON mode serializes canvas cells with coordinates and hex colors. Supports `--region` for subregion preview and `--color-format` for ANSI color depth.

**Acceptance Criteria**:
- `kakukuma preview test.kaku` renders ANSI art to stdout (colored terminal output)
- `kakukuma preview test.kaku --format json` outputs valid JSON with width, height, cells array, non_empty_count
- JSON cells include `x`, `y`, `fg` (hex string or null), `bg` (hex string or null), `char`
- `--region 0,0,10,10` limits output to specified bounding box
- `--color-format 256` uses xterm-256 escape codes
- `--color-format 16` uses ANSI 16-color codes
- `--color-format truecolor` uses 24-bit true color (default)
- Preview is read-only — does not modify .kaku or .kaku.log files
- Empty canvas produces empty ANSI output or JSON with `non_empty_count: 0`
- Unit tests: JSON output format validation, region filtering, color format mapping

**Estimated effort**: Medium
**Dependencies**: Task 1.4

---

### Task 1.7: TUI regression verification

**Description**: Verify that all existing TUI functionality works identically after the main.rs restructure. Run all existing tests. Manually verify TUI launch, file open, drawing, save, and exit.

**Acceptance Criteria**:
- `cargo test` — all 65+ existing tests pass
- `cargo run` launches TUI with no visual or behavioral changes
- `cargo run -- existing.kaku` opens file correctly
- Drawing with all tools works in TUI
- Save/load/export works in TUI
- Undo/redo works in TUI
- Theme cycling works in TUI
- Help dialog shows correctly
- No new compiler warnings

**Estimated effort**: Small
**Dependencies**: Tasks 1.1-1.6

---

## Sprint 2: Inspection, Analysis & History ✅

**Goal**: An agent can inspect canvas state, undo/redo operations, compare canvases, and get statistics. This completes the agent's analytical capabilities.

**Global ID**: sprint-2

### Task 2.1: Implement `inspect` command

**Description**: Implement `src/cli/inspect.rs` with four query modes: single cell, region, row, and column. All output as JSON. Read-only — no file modifications.

**Acceptance Criteria**:
- `kakukuma inspect test.kaku 5,5` returns `{"x":5,"y":5,"fg":"#FF0000","bg":null,"char":"█","empty":false}`
- `kakukuma inspect test.kaku --region 0,0,10,10` returns JSON array of non-empty cells in region
- `kakukuma inspect test.kaku --row 5` returns JSON array of all cells in row 5
- `kakukuma inspect test.kaku --col 5` returns JSON array of all cells in column 5
- Empty cells report `"empty": true`
- Out-of-bounds coordinates produce exit code 1 with descriptive error
- Read-only — does not modify .kaku or .kaku.log
- Unit tests: each query mode, empty canvas, out-of-bounds, non-empty filtering

**Estimated effort**: Medium
**Dependencies**: Sprint 1 complete

---

### Task 2.2: Implement `undo` and `redo` commands

**Description**: Implement undo/redo using the operation log. `undo` reads the last N active log entries, applies inverse mutations to canvas, moves the undo pointer. `redo` reads the last N undone entries, re-applies mutations, moves pointer. Both save the canvas atomically.

**Acceptance Criteria**:
- `kakukuma undo test.kaku` reverses the last drawing operation
- `kakukuma undo test.kaku --count 3` reverses the last 3 operations
- `kakukuma redo test.kaku` re-applies the last undone operation
- `kakukuma redo test.kaku --count 3` re-applies last 3 undone operations
- Undo with no operations produces exit code 1 with "Nothing to undo"
- Redo with no undone operations produces exit code 1 with "Nothing to redo"
- New draw after undo clears redo stack (undone entries truncated)
- Canvas state matches expected after undo/redo sequences
- Outputs `{"ok":true,"undone":1,"remaining":N}` or `{"ok":true,"redone":1,"remaining":N}`
- Unit tests: single undo, multi-undo, redo after undo, redo cleared by new draw, undo on empty log

**Estimated effort**: Medium
**Dependencies**: Sprint 1 complete (oplog module)

---

### Task 2.3: Implement `history` command

**Description**: Implement `src/cli/history_cmd.rs` to display the operation log. Summary mode shows timestamped one-line entries. Full mode includes mutation details.

**Acceptance Criteria**:
- `kakukuma history test.kaku` shows numbered list of operations with timestamps and cell counts
- Undone entries marked with `[undone]`
- `kakukuma history test.kaku --full` includes JSON mutation details per entry
- Empty log shows "No operations recorded"
- Output format is human-readable text (not JSON) by default
- Unit tests: formatting, empty log, undone markers

**Estimated effort**: Small
**Dependencies**: Task 2.2

---

### Task 2.4: Implement `diff` command

**Description**: Implement `src/cli/diff.rs` for canvas comparison. Two-file mode compares two `.kaku` files cell by cell. `--before` mode compares current state against state before the last operation (using the log).

**Acceptance Criteria**:
- `kakukuma diff a.kaku b.kaku` outputs JSON with changes array, added/removed/modified/unchanged counts
- Handles different canvas dimensions (pads smaller canvas with empty cells)
- `kakukuma diff test.kaku --before` shows what changed in the last operation
- `--before` with no log produces exit code 1 with descriptive error
- Identical canvases produce `{"changes":[],"added":0,"removed":0,"modified":0,"unchanged":N}`
- Change entries include `x`, `y`, `before`, `after` with cell data
- Unit tests: identical files, single change, multiple changes, different dimensions, --before mode

**Estimated effort**: Medium
**Dependencies**: Sprint 1 complete (oplog for --before)

---

### Task 2.5: Implement `stats` command

**Description**: Implement `src/cli/stats.rs` for canvas statistics. Computes fill percentage, color distribution, character distribution, bounding box, and symmetry scores.

**Acceptance Criteria**:
- `kakukuma stats test.kaku` outputs JSON matching the contract in SDD section 6.6
- `canvas` section: width, height, total_cells
- `fill` section: empty, filled, fill_percent (2 decimal places)
- `colors` section: unique_fg, unique_bg, distribution sorted by count descending (hex color, count, percent)
- `characters` section: unique, distribution sorted by count descending (char, count, percent)
- `bounding_box`: min_x, min_y, max_x, max_y (null if canvas is empty)
- `symmetry_score`: horizontal (0.0-1.0), vertical (0.0-1.0) — pixel-wise mirror comparison
- Empty canvas: fill_percent 0, bounding_box null, symmetry_score {horizontal: 1.0, vertical: 1.0}
- Unit tests: empty canvas, partially filled, symmetric patterns, color distribution accuracy

**Estimated effort**: Medium
**Dependencies**: Sprint 1 complete

---

## Sprint 3: Palette, Integration Tests & Polish ✅

**Goal**: Complete feature parity with palette management, add comprehensive integration tests, and polish error handling. The agent mode is production-ready.

**Global ID**: sprint-3

### Task 3.1: Implement `palette` command

**Description**: Implement `src/cli/palette_cmd.rs` with subcommands for palette and theme access. Expose theme data from `theme.rs` with a `pub fn all_themes()` accessor.

**Acceptance Criteria**:
- `kakukuma palette list` lists `.palette` files from current directory
- `kakukuma palette show NAME` outputs palette colors as JSON array with hex values
- `kakukuma palette create "My Palette" art.kaku` extracts unique colors from canvas, saves as `.palette`
- `kakukuma palette export NAME --output out.json` copies palette file to specified path
- `kakukuma palette add NAME "#FF6600"` appends color to existing palette
- `kakukuma palette themes` lists available theme names (warm, neon, dark)
- `kakukuma palette theme warm` shows theme UI colors as JSON
- `theme.rs` modified to add `pub fn all_themes()` accessor
- Unit tests: list, show, create from canvas, add color, theme listing

**Estimated effort**: Medium
**Dependencies**: Sprint 2 complete

---

### Task 3.2: Implement `export` CLI command

**Description**: Add a CLI `export` subcommand that wraps the existing export functionality. Supports ANSI and plain text output to file (distinct from `preview` which goes to stdout).

**Acceptance Criteria**:
- `kakukuma export test.kaku --output art.ans` exports ANSI art to file
- `kakukuma export test.kaku --output art.ans --color-format 256` uses 256-color
- `kakukuma export test.kaku --output art.txt --format plain` exports plain Unicode (no color)
- Outputs `{"exported":"art.ans","format":"ansi","color_format":"truecolor"}` to stdout
- Unit tests: file creation, format selection

**Estimated effort**: Small
**Dependencies**: Sprint 1 preview command (shared code)

---

### Task 3.3: Integration tests

**Description**: Create Cargo integration tests in `tests/` directory that invoke the compiled binary via `std::process::Command` and validate stdout, stderr, and exit codes for end-to-end workflows.

**Acceptance Criteria**:
- `tests/cli_new.rs`: create canvas, verify file contents, dimension clamping, force overwrite
- `tests/cli_draw.rs`: draw with each tool, verify canvas state via inspect
- `tests/cli_preview.rs`: preview ANSI output is non-empty, JSON output is valid, region filtering works
- `tests/cli_roundtrip.rs`: new → draw → preview → inspect → verify consistency across commands
- `tests/cli_undo_redo.rs`: draw → undo → verify state → redo → verify state
- `tests/cli_diff.rs`: create two files, draw differently, diff, verify output
- `tests/cli_stats.rs`: draw known patterns, verify exact statistics
- `tests/cli_symmetry.rs`: draw with all symmetry modes, verify mirrored cells via inspect
- All integration tests pass in CI (`cargo test`)
- Tests clean up temporary files after execution

**Estimated effort**: Large
**Dependencies**: Sprints 1-2 complete, Tasks 3.1-3.2 complete

---

### Task 3.4: Error handling polish and edge cases

**Description**: Review all CLI commands for consistent error handling. Ensure all errors have descriptive messages, correct exit codes (1 for user errors, 2 for internal), and that edge cases (empty canvas, max-size canvas, corrupt files) are handled gracefully.

**Acceptance Criteria**:
- All error messages printed to stderr (not stdout)
- Exit code 1 for: file not found, invalid coordinates, invalid color, file already exists, nothing to undo/redo
- Exit code 2 for: file corruption, I/O failures, serialization errors
- Corrupt `.kaku` file produces descriptive error (not panic)
- Corrupt `.kaku.log` file produces descriptive error, operation continues without log
- Out-of-bounds coordinates show canvas dimensions in error message
- Invalid hex color shows expected format in error message
- All error paths have test coverage

**Estimated effort**: Medium
**Dependencies**: Tasks 3.1-3.3

---

### Task 3.5: Performance validation

**Description**: Measure actual performance of CLI operations against the targets in the SDD. Profile hot paths (file I/O, JSON serialization). Document results.

**Acceptance Criteria**:
- `new` on 48x32: < 50ms
- `draw pencil` on 48x32: < 100ms
- `draw line` on 48x32: < 200ms
- `draw fill` on 128x128 (entire canvas): < 500ms
- `preview --format ansi` on 48x32: < 200ms
- `preview --format json` on 48x32: < 100ms
- `inspect` single cell: < 50ms
- `diff` two 48x32 files: < 200ms
- `stats` on 48x32: < 200ms
- `undo` on 48x32: < 200ms
- Results documented in NOTES.md
- Any operation exceeding target has an identified optimization path

**Estimated effort**: Small
**Dependencies**: All implementation tasks complete

---

## Summary

| Sprint | Tasks | Focus |
|--------|-------|-------|
| **Sprint 1** | 7 tasks | Foundation: clap, oplog, CLI scaffold, new, draw (all 6 tools), preview, TUI regression |
| **Sprint 2** | 5 tasks | Analysis: inspect, undo/redo, history, diff, stats |
| **Sprint 3** | 5 tasks | Polish: palette, export CLI, integration tests, error handling, performance |

### Dependencies

```
Sprint 1: 1.1 → 1.3 → 1.4 → 1.5, 1.6
           1.2 → 1.4
           1.1-1.6 → 1.7

Sprint 2: All of Sprint 1 → 2.1, 2.2, 2.4, 2.5
           2.2 → 2.3

Sprint 3: Sprint 2 → 3.1, 3.2
           3.1, 3.2 → 3.3
           3.3 → 3.4
           All → 3.5
```

### Risk Buffer

Each sprint has a 20% buffer built into the task estimates. The main risk is clap integration with the existing argument parsing in Sprint 1 — this should be validated first in Task 1.1.
