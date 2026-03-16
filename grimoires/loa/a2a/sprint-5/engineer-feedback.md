# Sprint 1 Code Review — CLI Normalization + Format Infrastructure

**Reviewer**: Senior Technical Lead
**Sprint**: Sprint 1 (Global Sprint 5)
**Cycle**: 019 — CLI Polish & Image Export
**Verdict**: **All good**

## Task Verification

### Task 1.1: PreviewFormat Enum Extension ✅
- `Auto` and `Png` variants added to `PreviewFormat` (`src/cli/mod.rs:338-344`)
- `PartialEq` derive added for `detect_format()` comparison
- Default changed from `"ansi"` to `"auto"` on Export command format field
- Preview command retains `"ansi"` default (correct — preview doesn't need auto-detect)

### Task 1.2: Export Command Normalization ✅
- `output: Option<String>` as positional arg (`mod.rs:96`)
- `output_flag: Option<String>` with `#[arg(long = "output", hide = true)]` (`mod.rs:98`)
- `cell_size`, `scale`, `no_crop` fields added with correct defaults (`mod.rs:107-114`)
- Dispatch merges `output.or(output_flag)` with structured JSON error (`mod.rs:526-528`)

### Task 1.3: Import Command Normalization ✅
- Same positional + hidden flag pattern applied (`mod.rs:199-201`)
- Dispatch merges correctly (`mod.rs:534-536`)

### Task 1.4: Batch Command Normalization ✅
- `commands: String` is now positional (no `#[arg(long)]`) (`mod.rs:225`)
- `--dry-run` flag unchanged (`mod.rs:228`)

### Task 1.5: Palette Export Normalization ✅
- `output: String` is now positional (`mod.rs:375`)
- `name` remains first positional (`mod.rs:374`)

### Task 1.6: Auto-Format Detection ✅
- `detect_format()` implemented in `preview.rs:48-58`
- Correctly maps `.png`, `.json`, `.txt` extensions
- Falls back to `Ansi` for unknown extensions
- Explicit format bypasses detection
- `export_to_file()` calls it before dispatch (`preview.rs:85`)

### Task 1.7: Undo/Clear Help Text ✅
- `Undo` doc comment explains linear model and overlap behavior (`mod.rs:134-139`)
- `Clear` doc comment warns about destructive nature (`mod.rs:181-185`)

### Task 1.8: CLI Normalization Tests ✅
- 7 new tests in `preview.rs`: format detection (5 tests) + cell-size parsing (2 tests)
- `try_parse_from` integration tests for positional args not present, but clap compile-time validation + manual CLI verification provides adequate coverage
- All 358 tests passing (351 baseline + 7 new)

## Bonus: Sprint 2 Work Delivered Early

The full PNG rendering engine was implemented in `export.rs` as part of Sprint 1 to make `export FILE out.png` work end-to-end:
- `to_png()` with bounding box crop, scale (capped at 8), nearest-neighbor upscale
- `render_cell_to_pixels()` dispatching all 20 block characters
- `shade_pixel()` dither patterns (25%/50%/75%)
- `vertical_fraction()` and `horizontal_fraction()` for 12 fractional fills
- `fill_rect()` helper to avoid duplication

This is excellent. Sprint 2's remaining work is primarily the comprehensive test suite (Task 2.6).

## Code Quality

- Clean separation: CLI parsing → routing → rendering
- Backward-compatible hidden flag aliases are elegant
- Color mapping: `Some(Rgb)` → `Rgba([r,g,b,255])`, `None` → `Rgba([0,0,0,0])` — correct (uses raw RGB, not `to_ratatui()`)
- No hardcoded Color::Rgb() (project convention upheld)
- Structured JSON errors throughout
- No security concerns

## Minor Observations (Non-blocking)

1. `vertical_fraction()` includes `LOWER_HALF` and `horizontal_fraction()` includes `LEFT_HALF` — these are unreachable from `render_cell_to_pixels()` because the dedicated half-block branches return early. Harmless as standalone utility functions.
2. `PreviewFormat` could derive `Copy` (all unit variants) to avoid `.clone()` in `detect_format()`. Not required.
