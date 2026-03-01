# Sprint Plan: CLI Polish & Image Export

> **Cycle**: 019
> **Created**: 2026-03-01
> **PRD**: grimoires/loa/prd.md
> **SDD**: grimoires/loa/sdd.md

---

## Sprint Overview

| Item | Value |
|------|-------|
| Total sprints | 2 |
| Sprint 1 | CLI Normalization + Format Infrastructure |
| Sprint 2 | PNG Export Engine |
| Estimated new tests | 25 |
| Target total tests | 381+ |
| New files | 0 |
| Modified files | 3 (`src/cli/mod.rs`, `src/export.rs`, `src/cli/preview.rs`) |

---

## Sprint 1: CLI Normalization + Format Infrastructure

**Goal**: Normalize all 4 commands with required flags to positional args, add backward-compatible aliases, add `Auto`/`Png` format variants, wire up new export args (`--cell-size`, `--scale`, `--no-crop`), and update undo/clear help text. This sprint sets up all CLI plumbing for Sprint 2's PNG rendering.

### Task 1.1: PreviewFormat Enum Extension

**Description**: Add `Auto` and `Png` variants to the `PreviewFormat` enum in `src/cli/mod.rs`. `Auto` becomes the new default for `--format`.

**Files**: `src/cli/mod.rs`

**Acceptance Criteria**:
- [x] `PreviewFormat::Auto` variant added
- [x] `PreviewFormat::Png` variant added
- [x] `#[arg(long, default_value = "auto")]` on format field (was `"ansi"`)
- [x] Compiles with no warnings

### Task 1.2: Export Command Normalization

**Description**: Convert `Export` command's `--output` required flag to a positional arg. Add hidden `--output` alias for backward compatibility. Add new PNG-related args (`--cell-size`, `--scale`, `--no-crop`).

**Files**: `src/cli/mod.rs`

**Acceptance Criteria**:
- [x] `output: Option<String>` as positional arg (no `#[arg(long)]`)
- [x] `output_flag: Option<String>` with `#[arg(long = "output", hide = true)]` for backward compat
- [x] `cell_size: String` with `#[arg(long, default_value = "8x16")]`
- [x] `scale: u32` with `#[arg(long, default_value_t = 1)]`
- [x] `no_crop: bool` with `#[arg(long)]`
- [x] Dispatch merges `output.or(output_flag)` with clear error if both missing

### Task 1.3: Import Command Normalization

**Description**: Convert `Import` command's `--output` required flag to a positional arg with hidden flag alias.

**Files**: `src/cli/mod.rs`

**Acceptance Criteria**:
- [x] `output: Option<String>` as positional arg
- [x] `output_flag: Option<String>` with `#[arg(long = "output", hide = true)]`
- [x] Dispatch merges `output.or(output_flag)` with clear error if both missing

### Task 1.4: Batch Command Normalization

**Description**: Convert `Batch` command's `--commands` required flag to a positional arg. No backward compat alias needed (newer command, Approach B per SDD).

**Files**: `src/cli/mod.rs`

**Acceptance Criteria**:
- [x] `commands: String` as positional arg (no `#[arg(long)]`)
- [x] Existing `--dry-run` flag unchanged
- [x] Routing to `batch::run_batch()` unchanged

### Task 1.5: Palette Export Normalization

**Description**: Convert `PaletteAction::Export`'s `--output` required flag to a positional arg. No backward compat alias (Approach B).

**Files**: `src/cli/mod.rs`

**Acceptance Criteria**:
- [x] `output: String` as positional arg (no `#[arg(long)]`)
- [x] `name` remains first positional, `output` becomes second positional

### Task 1.6: Auto-Format Detection

**Description**: Implement `detect_format()` function in `src/cli/preview.rs` that infers format from output file extension when `--format auto`.

**Files**: `src/cli/preview.rs`

**Acceptance Criteria**:
- [x] `detect_format(output, explicit)` function implemented
- [x] `.png` → `PreviewFormat::Png`
- [x] `.json` → `PreviewFormat::Json`
- [x] `.txt` → `PreviewFormat::Plain`
- [x] Anything else → `PreviewFormat::Ansi` (default fallback)
- [x] Explicit format (not Auto) bypasses detection
- [x] `export_to_file()` calls `detect_format()` before dispatching

### Task 1.7: Undo/Clear Help Text

**Description**: Update doc comments on `Undo` and `Clear` command variants to document the linear undo model and clear's destructive behavior.

**Files**: `src/cli/mod.rs`

**Acceptance Criteria**:
- [x] `Undo` doc comment explains linear model and overlap behavior
- [x] `Clear` doc comment warns about destructive nature and undo limitations
- [x] Both visible in `kakukuma undo --help` and `kakukuma clear --help`

### Task 1.8: CLI Normalization Tests

**Description**: Unit tests validating positional args work and backward-compatible flags still work.

**Files**: `src/cli/mod.rs` (test module)

**Acceptance Criteria**:
- [x] Test: `export FILE OUTPUT --format plain` works (positional output)
- [x] Test: `export FILE --output OUTPUT --format plain` still works (flag compat)
- [x] Test: `import IMAGE OUTPUT` works (positional output)
- [x] Test: `import IMAGE --output OUTPUT` still works (flag compat)
- [x] Test: `batch FILE COMMANDS` works (positional commands)
- [x] Test: auto-format detects `.png` as PNG
- [x] Test: auto-format detects `.txt` as plain
- [x] Test: auto-format falls back to ANSI for unknown extension
- [x] All 358 tests pass (351 baseline + 7 new)

---

## Sprint 2: PNG Export Engine

**Goal**: Implement the `to_png()` rendering function with full block character support (all 20 chars), shade dithering, fractional fills, auto-crop, scaling, and wire it through the CLI. This sprint delivers the image export capability.

### Task 2.1: Core to_png Function

**Description**: Implement `to_png()` in `src/export.rs` with the rendering pipeline: bounding box → pixel grid → cell rendering → scale.

**Files**: `src/export.rs`

**Acceptance Criteria**:
- [x] `pub fn to_png(canvas, cell_w, cell_h, scale, crop) -> image::RgbaImage` signature
- [x] Uses existing `bounding_box()` when `crop` is true
- [x] Creates `RgbaImage::new(region_w * cell_w, region_h * cell_h)`
- [x] Iterates cells in region, calls `render_cell_to_pixels()` for each
- [x] If `scale > 1`, upscales with `image::imageops::resize()` + `FilterType::Nearest`
- [x] Scale capped at 8 (reject >8)

### Task 2.2: Block Character Pixel Rendering

**Description**: Implement `render_cell_to_pixels()` that fills a cell's pixel block based on its character type. Handles all 5 primary block characters.

**Files**: `src/export.rs`

**Acceptance Criteria**:
- [x] `render_cell_to_pixels(img, cell, px, py, cw, ch)` function
- [x] `█` (FULL_BLOCK): entire block filled with fg color
- [x] `▀` (UPPER_HALF): top half fg, bottom half bg/transparent
- [x] `▄` (LOWER_HALF): bottom half fg, top half bg/transparent
- [x] `▌` (LEFT_HALF): left half fg, right half bg/transparent
- [x] `▐` (RIGHT_HALF): right half fg, left half bg/transparent
- [x] Space/empty: fill with bg, or transparent if no bg
- [x] Any other printable char: fill entire block with fg (simplified)
- [x] Color mapping: `Some(Rgb)` → `Rgba([r,g,b,255])`, `None` → `Rgba([0,0,0,0])`

### Task 2.3: Shade Dither Patterns

**Description**: Implement `shade_pixel()` function for the 3 shade characters using repeating dither patterns.

**Files**: `src/export.rs`

**Acceptance Criteria**:
- [x] `shade_pixel(x, y, shade) -> bool` function
- [x] `░` (SHADE_LIGHT): ~25% fg density using `(x + y) % 4 == 0`
- [x] `▒` (SHADE_MEDIUM): ~50% fg density using `(x + y) % 2 == 0` checkerboard
- [x] `▓` (SHADE_DARK): ~75% fg density using `(x + y) % 4 != 0`
- [x] Integrated into `render_cell_to_pixels()` dispatch

### Task 2.4: Fractional Fill Rendering

**Description**: Implement `vertical_fraction()` and `horizontal_fraction()` for the 12 fractional block characters.

**Files**: `src/export.rs`

**Acceptance Criteria**:
- [x] `vertical_fraction(ch) -> f32` for `▁▂▃▅▆▇` (1/8 through 7/8)
- [x] `horizontal_fraction(ch) -> f32` for `▉▊▋▍▎▏` (7/8 through 1/8)
- [x] Vertical: pixel is fg when `y_in_cell >= cell_h * (1.0 - fraction)` (fills from bottom)
- [x] Horizontal: pixel is fg when `x_in_cell < cell_w * fraction` (fills from left)
- [x] Integrated into `render_cell_to_pixels()` dispatch

### Task 2.5: CLI Routing + Cell Size Parsing

**Description**: Add `PreviewFormat::Png` case to `export_to_file()` in `src/cli/preview.rs`. Implement `parse_cell_size()` for the `--cell-size` argument.

**Files**: `src/cli/preview.rs`

**Acceptance Criteria**:
- [x] `parse_cell_size("8x16") -> Ok((8, 16))` function
- [x] Rejects 0 or >64 for either dimension
- [x] Rejects non-numeric or missing `x` separator
- [x] `PreviewFormat::Png` case calls `export::to_png()` and `img.save()`
- [x] JSON output: `{"exported": "...", "format": "png", "width": N, "height": N, "cell_size": "WxH"}`
- [x] PNG export args (`cell_size`, `scale`, `no_crop`) threaded from Command to export_to_file

### Task 2.6: PNG Export Tests

**Description**: Comprehensive tests for the PNG rendering engine covering all character types, colors, transparency, crop, and scale.

**Files**: `src/export.rs` (test module)

**Acceptance Criteria**:
- [x] Test: empty canvas produces fully transparent PNG
- [x] Test: single cell with `█` fills entire cell block with fg
- [x] Test: `▀` fills top half fg, bottom half bg/transparent
- [x] Test: `▄` fills bottom half fg, top half bg/transparent
- [x] Test: `▌` fills left half fg, right half bg/transparent
- [x] Test: `▐` fills right half fg, left half bg/transparent
- [x] Test: `░` produces ~25% fg pixel density
- [x] Test: `▒` produces ~50% fg pixel density
- [x] Test: `▓` produces ~75% fg pixel density
- [x] Test: `▂` (LOWER_1_4) fills bottom quarter with fg
- [x] Test: `▊` (LEFT_3_4) fills left three-quarters with fg
- [x] Test: auto-crop exports only bounding box region
- [x] Test: no-crop exports full canvas dimensions
- [x] Test: scale 2x produces doubled dimensions with nearest-neighbor
- [x] Test: cells with `bg: None` have alpha=0 pixels
- [x] Test: custom cell-size 4x8 produces smaller pixel blocks
- [x] Test: cell_size parsing rejects "0x16" and "abc"
- [x] All 375 tests pass (358 sprint-1 + 17 new PNG tests)

---

## Risk Assessment

| Risk | Sprint | Mitigation |
|------|--------|------------|
| Clap rejects two fields targeting same `--output` name | Sprint 1 | Approach A uses `#[arg(long = "output")]` on hidden field — test first with `try_parse_from` |
| `image::imageops::resize` not available with current features | Sprint 2 | Already used by import.rs for reference layer — confirmed available |
| Block character constants not accessible from export.rs | Sprint 2 | Constants in `src/cell.rs` are `pub` — import directly |
| Large canvas + high scale causes OOM | Sprint 2 | Cap scale at 8, validate cell_size max 64x64 (per SDD §8) |
