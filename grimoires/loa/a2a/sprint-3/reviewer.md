# Sprint 3 Implementation Report

**Sprint**: sprint-3
**Date**: 2026-02-14
**Status**: Ready for review

---

## Summary

Sprint 3 completes the agent mode with palette management, comprehensive integration tests, error handling polish, and performance validation. All 5 tasks are complete. The CLI is production-ready.

---

## Task Completion

### Task 3.1: Implement `palette` command

**Status**: Complete (implemented in Sprint 1, verified in Sprint 3)

**File**: `src/cli/palette_cmd.rs` (225 lines)

7 subcommands implemented:
- `list` — lists default palette + custom `.palette` files from current directory
- `show NAME` — outputs palette colors as JSON with hex, r, g, b
- `create NAME file.kaku` — extracts unique colors from canvas, saves as `.palette`
- `export NAME --output out.json` — copies palette file to output path
- `add NAME "#FF6600"` — appends color to existing palette
- `themes` — lists available theme names (Warm, Neon, Dark)
- `theme NAME` — shows theme UI colors as JSON with case-insensitive matching

**AC deviation**: The sprint plan calls for `pub fn all_themes()` in `theme.rs`, but the existing `pub const THEMES: [Theme; 3]` is used directly. Functionally equivalent — data is publicly accessible.

### Task 3.2: Implement `export` CLI command

**Status**: Complete (implemented in Sprint 1, verified in Sprint 3)

**File**: `src/cli/preview.rs` (lines 42-77, `export_to_file()`)

- Writes to file (vs `preview` which outputs to stdout)
- Supports ANSI, plain text, JSON formats
- Supports `--color-format` for 256/16/truecolor
- Outputs `{"exported":"art.ans","format":"ansi","color_format":"truecolor"}` to stdout

### Task 3.3: Integration tests

**Status**: Complete (new in Sprint 3)

**Files**: 8 test files in `tests/` directory

| Test File | Tests | Coverage |
|-----------|-------|----------|
| `tests/helpers.rs` | 0 | Shared utilities: `kakukuma()`, `temp_file()`, `run_ok()`, `stdout_json()`, `cleanup()` |
| `tests/cli_new.rs` | 6 | create, custom dimensions, clamping, exists check, force overwrite, log creation |
| `tests/cli_draw.rs` | 9 | pencil, eraser, line, rect outline, rect filled, fill, eyedropper, invalid coords, invalid color |
| `tests/cli_preview.rs` | 4 | ANSI non-empty, JSON valid, region filtering, plain no escapes |
| `tests/cli_roundtrip.rs` | 2 | new-draw-inspect consistency, draw-export-verify |
| `tests/cli_undo_redo.rs` | 5 | undo, redo, multi-undo, redo cleared by new draw, undo on empty |
| `tests/cli_diff.rs` | 4 | identical files, changes detected, --before mode, empty log fails |
| `tests/cli_stats.rs` | 3 | empty canvas, content with distributions, symmetry scores |
| `tests/cli_symmetry.rs` | 3 | horizontal, vertical, quad mirror verification via inspect |

**Total**: 36 integration tests. All pass. All use unique temp files and clean up after execution.

### Task 3.4: Error handling polish

**Status**: Complete (new in Sprint 3)

**Fix applied**: `resolve_colors()` in `src/cli/mod.rs` (lines 339-358)

**Before**: Invalid hex colors silently fell through to white default. `parse_hex_color` returned `None`, and `fg.or(Some(Rgb::WHITE))` replaced it with white.

**After**: Invalid hex colors now produce a descriptive error with exit code 1:
```
Error: Invalid hex color 'not-a-color'. Expected format: #RRGGBB (e.g. #FF0000)
```

**Error handling audit results**:
- All error messages → stderr via `eprintln!`: PASS
- Exit code 1 for user errors (file not found, invalid coords, invalid color, file exists, nothing to undo/redo): PASS
- Exit code 2 for internal errors (file corruption, I/O failures): PASS
- OOB coords show canvas dimensions: PASS (e.g., "Position (100, 100) exceeds canvas dimensions (16x16)")
- Invalid hex color shows expected format: PASS (fixed in this sprint)
- Integration test `draw_invalid_color_fails` verifies the error message

### Task 3.5: Performance validation

**Status**: Complete

All operations measured against SDD targets using release build:

| Operation | Target | Actual | Margin |
|-----------|--------|--------|--------|
| `new` 48x32 | <50ms | ~6ms | 8x under |
| `draw pencil` 48x32 | <100ms | ~6ms | 17x under |
| `draw line` 48x32 | <200ms | ~6ms | 33x under |
| `draw fill` 128x128 | <500ms | ~19ms | 26x under |
| `preview ansi` 48x32 | <200ms | ~5ms | 40x under |
| `preview json` 48x32 | <100ms | ~6ms | 17x under |
| `inspect` single cell | <50ms | ~6ms | 8x under |
| `diff` two 48x32 | <200ms | ~5ms | 40x under |
| `stats` 48x32 | <200ms | ~5ms | 40x under |
| `undo` 48x32 | <200ms | ~6ms | 33x under |

All operations are 8-40x under target. No optimizations needed.

---

## Test Results

```
running 211 tests (unit)
test result: ok. 211 passed; 0 failed; 0 ignored

running 36 tests (integration, 8 test files)
test result: ok. 36 passed; 0 failed; 0 ignored

Total: 247 tests, 0 failures, 0 warnings
```

---

## Files Created/Modified in Sprint 3

| File | Lines | Status |
|------|-------|--------|
| `src/cli/mod.rs` | 555 | Modified: `resolve_colors()` now validates hex colors |
| `tests/helpers.rs` | 36 | Created: shared test utilities |
| `tests/cli_new.rs` | 59 | Created: 6 tests |
| `tests/cli_draw.rs` | 159 | Created: 9 tests |
| `tests/cli_preview.rs` | 50 | Created: 4 tests |
| `tests/cli_roundtrip.rs` | 62 | Created: 2 tests |
| `tests/cli_undo_redo.rs` | 95 | Created: 5 tests |
| `tests/cli_diff.rs` | 65 | Created: 4 tests |
| `tests/cli_stats.rs` | 68 | Created: 3 tests |
| `tests/cli_symmetry.rs` | 64 | Created: 3 tests |

## Architecture Notes

- Integration tests use `env!("CARGO_BIN_EXE_kakukuma")` for the binary path — works correctly in `cargo test`
- Each test gets a unique temp file via `AtomicUsize` counter to prevent parallel test collisions
- All temp files cleaned up after each test via `cleanup()` helper
- `helpers.rs` compiled as its own test crate (0 tests) but also used as `mod helpers;` in other test files
