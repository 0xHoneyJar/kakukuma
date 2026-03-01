# Sprint 2 Implementation Report (Global Sprint-4)

## Cycle 018: Creative Power Tools — Reference Layer + Integration Polish

### Summary

Sprint 2 implements the Reference Layer feature (FR-2) for both TUI and CLI, along with integration polish. Users can load a reference/trace image behind their canvas with adjustable transparency, enabling tracing workflows for pixel art creation. All 6 tasks completed. 13 new tests added (356 total, all passing).

### Tasks Completed

#### Task 2.1: Project File v6 — Reference Image Persistence
**File**: `src/project.rs`
- Added `reference_image: Option<String>` field to `Project` struct
- Used `#[serde(skip_serializing_if = "Option::is_none")]` and `#[serde(default)]` for backward compatibility
- `save_to_file()` auto-detects version 6 when reference_image is present
- `load_from_file()` accepts versions 1-6
- **Tests added**: 3 (v6 roundtrip, v5 backward compat, v5 stays v5)

#### Task 2.2: ReferenceLayer Type + Image Loading
**File**: `src/app.rs`
- Added `ReferenceLayer` struct: `colors: Vec<Vec<Option<Rgb>>>`, `image_path`, `brightness: u8`, `visible: bool`
- Added `dim_color()` function with 3 brightness levels (25%, 50%, 75%)
- Added `reference_layer: Option<ReferenceLayer>` field to `App`
- Added `load_reference()` method using `image` crate (resize_exact to canvas size, alpha threshold at 128)
- Updated `load_project()` to load reference image when project contains one
- **Tests added**: 5 (dim_color x3 brightness levels, reference_layer_init, registry_reference_reachable)

#### Task 2.3: Reference CLI Command
**File**: `src/cli/mod.rs`
- Added `Command::Reference { file, image, clear }` variant
- Implemented `cmd_reference()`: validates image exists, computes relative path, atomic save
- JSON output: `{ "reference": "<path>", "file": "<file>" }` or `{ "reference": null, "file": "<file>" }` on clear

#### Task 2.4: Reference Rendering in Editor
**File**: `src/ui/editor.rs`
- Added `grid_or_reference_bg()` function: checks reference layer before falling back to `grid_bg()`
- Updated `resolve_half_block_for_display()` to accept `reference: Option<&ReferenceLayer>` parameter
- Reference colors are dimmed via `dim_color()` before rendering
- Transparent reference pixels (None) fall through to grid
- **Tests added**: 5 (without reference, visible reference, hidden reference, transparent pixel, half-block with reference)

#### Task 2.5: Reference Toggle + Brightness via Command Palette
**File**: `src/app.rs`
- Added "Toggle Reference" command (category: "Reference") — toggles `visible` flag
- Added "Reference Brightness" command — cycles through 3 brightness levels (Low/Med/High)
- Fixed borrow checker issue: extract message string before calling `app.set_status()`

#### Task 2.6: Integration Testing
- All 356 tests passing (202 lib + 102 bin + 52 integration)
- Baseline was 343 (Sprint 1), 13 new tests added
- No regressions

### Test Results

```
test result: ok. 202 passed; 0 failed (lib)
test result: ok. 102 passed; 0 failed (bin)
test result: ok.  52 passed; 0 failed (integration)
Total: 356 tests, 0 failures
```

### Build Warnings (pre-existing)

- `field image_path is never read` on ReferenceLayer (stored for persistence roundtrip)
- `field category is never read` on PaletteCommand (pre-existing from Sprint 1)
- `function build_spans is never used` in statusbar.rs (pre-existing)

### Files Changed

| File | Changes |
|------|---------|
| `src/project.rs` | +70 lines (Project v6, reference_image field, tests) |
| `src/app.rs` | +143 lines (ReferenceLayer, dim_color, load_reference, commands, tests) |
| `src/cli/mod.rs` | +57 lines (Reference command, cmd_reference) |
| `src/ui/editor.rs` | +128 lines (grid_or_reference_bg, rendering integration, tests) |
| `grimoires/loa/ledger.json` | Sprint-3 status updated to completed |

### Acceptance Criteria Status

- [x] Reference image loads and resizes to canvas dimensions
- [x] Three brightness levels for reference overlay (25%, 50%, 75%)
- [x] Toggle reference visibility on/off via command palette
- [x] CLI command to set/clear reference image with JSON output
- [x] Project file v6 with backward compatibility (v5 loads cleanly)
- [x] Transparent pixels in reference fall through to grid
- [x] All tests pass with no regressions
