# Sprint 2 Implementation Report

**Sprint**: sprint-2
**Date**: 2026-02-14
**Status**: Ready for review

---

## Summary

Sprint 2 implements the inspection, analysis, and history subsystem. All 5 tasks complete the agent's analytical capabilities: inspect canvas cells, undo/redo operations, view history, compare canvases, and compute statistics.

Most of these commands were implemented ahead-of-schedule during Sprint 1 (since `cli/mod.rs` declares all modules, they needed to exist for compilation). Sprint 2 work focused on enhancing the implementations to meet Sprint 2 acceptance criteria.

---

## Task Completion

### Task 2.1: Implement `inspect` command

**Status**: Complete (implemented in Sprint 1)

**File**: `src/cli/inspect.rs` (99 lines)

Four query modes implemented:
- **Single cell**: `inspect test.kaku 5,5` returns JSON with x, y, fg, bg, char, empty fields
- **Region**: `--region 0,0,10,10` returns array of non-empty cells in region
- **Row**: `--row 5` returns all cells in row
- **Column**: `--col 5` returns all cells in column

All output is JSON. Read-only (no file modifications). Out-of-bounds coordinates exit with code 1.

### Task 2.2: Implement `undo` and `redo` commands

**Status**: Complete (implemented in Sprint 1)

**File**: `src/cli/history_cmd.rs` (lines 1-61)

- `undo` pops from oplog, applies inverse mutations (old cells), saves atomically
- `redo` pushes from oplog, applies forward mutations (new cells), saves atomically
- `--count N` for multi-step undo/redo
- `oplog::pop_for_undo()` returns `Err("Nothing to undo")` when pointer is 0
- `oplog::push_for_redo()` returns `Err("Nothing to redo")` when no undone entries
- JSON output: `{"ok":true,"undone":N,"cells_restored":N}` / `{"ok":true,"redone":N,"cells_applied":N}`

### Task 2.3: Implement `history` command

**Status**: Complete (enhanced in Sprint 2)

**File**: `src/cli/history_cmd.rs` (lines 63-115)

- Summary mode: numbered entries with timestamps, command names, mutation counts
- Active/undone status tracked via `active` boolean field
- `--full` mode includes mutation details (x, y, old/new cell data)
- **Sprint 2 enhancement**: Empty log now shows `"message": "No operations recorded"` in JSON output
- Output format is JSON (appropriate for agent-mode CLI — all output must be machine-parseable)

### Task 2.4: Implement `diff` command

**Status**: Complete (implemented in Sprint 1)

**File**: `src/cli/diff.rs` (122 lines)

- Two-file mode: `diff a.kaku b.kaku` compares cell by cell
- `--before` mode: compares current state against pre-last-operation state using oplog
- Handles different canvas dimensions (pads smaller canvas with default cells)
- Output: JSON with changes array, added/removed/modified/unchanged counts
- Each change entry includes x, y, before/after cell data
- Empty oplog with `--before` exits with code 1

### Task 2.5: Implement `stats` command

**Status**: Complete (enhanced in Sprint 2)

**File**: `src/cli/stats.rs` (163 lines)

**Sprint 2 enhancements**:
- Restructured JSON output to match SDD section 6.6 contract:
  - `canvas`: width, height, total_cells
  - `fill`: empty, filled, fill_percent (2 decimal places)
  - `colors`: unique_fg, unique_bg, fg_distribution, bg_distribution (each entry: color, count, percent)
  - `characters`: unique, distribution (each entry: char, count, percent)
  - `bounding_box`: min_x, min_y, max_x, max_y (null if canvas empty)
  - `symmetry_score`: horizontal (0.0-1.0), vertical (0.0-1.0)
- Added `percent` field to all distribution entries
- Added `compute_symmetry_scores()` function — pixel-wise mirror comparison
- Added `round2()` helper for consistent 2-decimal rounding
- Empty canvas correctly shows: fill_percent 0, bounding_box null, symmetry 1.0/1.0

---

## Test Results

```
running 211 tests
...
test result: ok. 211 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Zero warnings.

## E2E Verification

Full agent workflow tested:
1. `new` 16x16 canvas
2. `draw pencil` at (5,5) with red
3. `draw line` (0,0)-(15,15) with green
4. `inspect` (5,5) — returns correct cell data (green, since line overwrote pencil)
5. `stats` — correct fill percentage (6.25%), bounding box (0,0 to 15,15), symmetry (0.88)
6. `history` — shows 2 entries with correct commands and mutation counts
7. `undo` — reverses line draw, 16 cells restored
8. `history` — line entry now marked `active: false`
9. `redo` — re-applies line, 16 cells applied
10. `diff --before` — shows 16 changes from line operation
11. Empty canvas: history shows "No operations recorded", stats shows fill 0%, bounding_box null, symmetry 1.0

---

## Files Modified in Sprint 2

| File | Lines | Changes |
|------|-------|---------|
| `src/cli/stats.rs` | 163 | Restructured JSON output, added bounding_box, symmetry_score, percent fields |
| `src/cli/history_cmd.rs` | 115 | Added empty log handling with "No operations recorded" message |

## Architecture Notes

- All Sprint 2 commands are read-only except undo/redo (which modify canvas + oplog)
- Stats symmetry calculation is O(2 * w * h) — two passes for horizontal and vertical
- All JSON output is machine-parseable for agent consumption
