# Sprint Plan: Import Fidelity & Color Intelligence

> **Cycle**: 022
> **Created**: 2026-03-06
> **PRD**: grimoires/loa/prd.md
> **SDD**: grimoires/loa/sdd.md

---

## Sprint Overview

| Item | Value |
|------|-------|
| Total sprints | 3 |
| Sprint 1 | Smart defaults + terminal color detection + `render` command |
| Sprint 2 | TUI import dialog enhancement + tests |
| Sprint 3 | Clipboard import, perceptual quantization, browse filter, UI polish |
| Estimated new tests | 10 (sprint 1-2) + 45 (sprint 3) |
| New files | 1 (`src/lib.rs`) |
| Modified files | 16 (see SDD for full list) |

---

## Sprint 1: Smart Defaults & Render Command

**Goal**: Make image-to-ANSI art work correctly out of the box. One command, no flags needed.

### Task 1.1: `ColorFormat::Auto` & `CliColorFormat::Auto`

**Description**: Add `Auto` variant to both enums. `Auto` is the new default for `preview`, `export` color-format args.

**Files**: `src/export.rs`, `src/cli/mod.rs`, `src/cli/preview.rs`

**Acceptance Criteria**:
- `ColorFormat::Auto` variant exists
- `CliColorFormat::Auto` variant exists with value string `"auto"`
- `to_color_format()` maps `CliColorFormat::Auto` → `ColorFormat::Auto`
- All match arms in export.rs handle `Auto` (resolve via `detect_terminal_colors()`)

**Tests**: 1 (Auto maps correctly)

### Task 1.2: `detect_terminal_colors()`

**Description**: Add function to `src/export.rs` that reads `COLORTERM` env var and returns appropriate `ColorFormat`.

**Files**: `src/export.rs`

**Logic**:
```
COLORTERM == "truecolor" | "24bit" → TrueColor
else → Color256Hue
```

**Acceptance Criteria**:
- Function returns `TrueColor` when `COLORTERM=truecolor`
- Function returns `TrueColor` when `COLORTERM=24bit`
- Function returns `Color256Hue` for any other value or unset
- Function is `pub` (used by CLI)

**Tests**: 3 (truecolor, 24bit, fallback)

### Task 1.3: Resolve `Auto` in ANSI Export Path

**Description**: Update `to_ansi()` and related export functions to resolve `ColorFormat::Auto` before use. Add a `resolve_color_format()` helper.

**Files**: `src/export.rs`

**Acceptance Criteria**:
- `resolve_color_format(Auto)` calls `detect_terminal_colors()`
- `resolve_color_format(explicit)` returns explicit unchanged
- `to_ansi()` calls resolve at entry point
- `emit_fg()`, `emit_bg()`, `emit_fg_bg()` never see `Auto` (resolved before dispatch)

**Tests**: 1 (resolve passthrough for explicit formats)

### Task 1.4: Smart Import Defaults

**Description**: Change `ImportOptions::default()` and CLI `Import` command defaults.

**Files**: `src/import.rs`, `src/cli/mod.rs`

**Changes**:
- `ImportOptions::default()`: `color_mode: TrueColor`, `normalize: true`, `preserve_hue: true`
- CLI `Import`: `--quantize` default `truecolor`, replace `--normalize`/`--preserve-hue` booleans with `--no-normalize`/`--no-preserve-hue` (inverted flags)
- CLI `Import`: `--color-format` default becomes `auto` (was `truecolor` hardcoded in `--quantize`)

**Acceptance Criteria**:
- `ImportOptions::default()` has truecolor, normalize=true, preserve_hue=true
- `kakukuma import img.png out.kaku` uses new defaults without any flags
- `--no-normalize` disables normalization
- `--no-preserve-hue` disables hue preservation
- Old `--normalize` and `--preserve-hue` flags removed (replaced by inverted versions)

**Tests**: 1 (default options verification)

### Task 1.5: Change Preview/Export Default Color Format

**Description**: Change default `--color-format` from `truecolor` to `auto` for both `Preview` and `Export` commands.

**Files**: `src/cli/mod.rs`

**Acceptance Criteria**:
- `Preview` command: `#[arg(long, default_value = "auto")]` for color_format
- `Export` command: same
- Auto-detection works for both commands

**Tests**: 0 (covered by task 1.2/1.3 tests)

### Task 1.6: `Render` Command

**Description**: New CLI subcommand that converts an image directly to ANSI on stdout. No intermediate file.

**Files**: `src/cli/mod.rs`

**Acceptance Criteria**:
- `kakukuma render image.png` outputs ANSI art to stdout
- `--width` and `--height` control output dimensions (defaults: 48x24)
- `--color-format auto` by default (terminal-detected)
- `--no-normalize`, `--no-preserve-hue`, `--boost` flags available
- Outputs JSON metadata to stderr: `{"width": N, "height": N, "color_format": "..."}`
- Returns error JSON if image not found or decode fails

**Tests**: 2 (success path, error path)

---

## Sprint 2: TUI Import Dialog & Polish

**Goal**: Expose smart defaults in the TUI and ensure everything is well-tested.

### Task 2.1: Update TUI Import Defaults

**Description**: Change hardcoded `ImportOpts` defaults in `src/input.rs` to match the new smart defaults.

**Files**: `src/input.rs`

**Acceptance Criteria**:
- `ImportOpts` constructed with `normalize: true`, `preserve_hue: true`, `color_boost: 1.0`
- TUI import uses same pipeline as CLI with smart defaults

**Tests**: 0 (behavioral, tested via manual TUI interaction)

### Task 2.2: TUI Import Dialog Toggles

**Description**: Add toggle UI for normalize and preserve-hue in the import options dialog.

**Files**: `src/input.rs`, `src/ui/mod.rs`

**Key bindings in import dialog**:
- `N` — toggle normalize (shows `[N]ormalize: ON/OFF`)
- `H` — toggle hue-preserve (shows `[H]ue preserve: ON/OFF`)

**Acceptance Criteria**:
- Import dialog shows current normalize/preserve-hue state
- N key toggles normalize
- H key toggles hue-preserve
- State is reflected in the import options passed to `import_image()`

**Tests**: 0 (TUI interaction)

### Task 2.3: Comprehensive Test Suite

**Description**: Add integration tests that verify the full pipeline with new defaults.

**Files**: `src/import.rs`, `src/export.rs`

**Tests**:
- `test_import_default_truecolor` — verify default import stores full RGB
- `test_render_auto_256` — verify render with no COLORTERM produces 256-color escapes
- `test_import_no_normalize` — verify --no-normalize produces different output

**Acceptance Criteria**:
- All new tests pass
- All existing tests pass (no regressions from default changes)
- `cargo test` clean

---

## Sprint 3: Clipboard Import, Perceptual Quantization & UI Polish

**Goal**: Complete the import UX pipeline — clipboard paste, perceptual color accuracy, browse filtering, and UI polish.

### Task 3.1: Clipboard Image Import (Ctrl+V)

**Description**: macOS clipboard integration via osascript (file paths) and arboard (raw pixels).

**Files**: `src/input.rs`, `src/app.rs`, `src/main.rs`

**Acceptance Criteria**:
- Ctrl+V opens import options with clipboard image as source
- Finder Cmd+C → Ctrl+V imports the file (not the icon)
- Screenshot Cmd+Shift+Ctrl+4 → Ctrl+V imports raw RGBA data
- `import_image_data()` handles raw RGBA clipboard data
- Bracketed paste enabled for paste detection

### Task 3.2: Import Browse Type-to-Filter

**Description**: Character filtering and path mode in the import file browser.

**Files**: `src/input.rs`, `src/ui/mod.rs`, `src/app.rs`

**Acceptance Criteria**:
- Typing characters filters the file list in real-time
- `/` or `~` enters path mode for direct path entry
- Tab completes matching files
- Backspace clears filter characters
- Esc clears filter or closes dialog

### Task 3.3: Perceptual Color Quantization

**Description**: Replace Euclidean RGB with perceptually-weighted distance in `nearest_256_hue()`.

**Files**: `src/cell.rs`, `src/import.rs`, `src/input.rs`

**Acceptance Criteria**:
- Redmean weighted distance formula
- Luminance preservation with asymmetric dark penalty
- Gray penalty increased to 20000
- Dark chromatic brightness lift in `boost_saturation()`
- Auto color_boost 1.2× for 256-color, 1.4× for 16-color

### Task 3.4: Full Blocks Import Fix

**Description**: Fix full-blocks rasterization to use `█` with fg instead of space with bg.

**Files**: `src/import.rs`

**Acceptance Criteria**:
- Full blocks import renders visible cells
- `is_empty()` returns false for imported cells
- Existing half-blocks tests unaffected

### Task 3.5: Import Options Dialog Expansion

**Description**: Add normalize, hue-preserve, and posterize controls to the TUI import dialog.

**Files**: `src/input.rs`, `src/ui/mod.rs`

**Acceptance Criteria**:
- Dialog shows 6 option rows (was 3)
- N toggles normalize, H toggles hue-preserve
- Posterize has 5 presets (Off, 8, 12, 16, 24)
- Source label shows "Clipboard image" for paste imports

### Task 3.6: Import Keymap Fix & UI Polish

**Description**: Fix Ctrl+I (Tab in terminals) → plain I key for import. Eyedropper → K. Status bar and toolbar updates.

**Files**: `src/input.rs`, `src/ui/statusbar.rs`, `src/ui/toolbar.rs`, `src/ui/mod.rs`

**Acceptance Criteria**:
- I key opens import, K key for eyedropper
- Status bar shows updated keybindings
- Toolbar shows 4-row block panel with shortcuts
- Block picker shows character info in footer

---

## Risk Notes

- **Existing test regressions**: Changing `ImportOptions::default()` may break tests that assumed Color256 default. Grep for `ImportOptions::default()` and `ImportOptions { ... }` in test code.
- **CLI flag inversion**: Switching from `--normalize` to `--no-normalize` is a breaking CLI change. Check if any scripts or docs reference old flags.
- **macOS-only clipboard**: osascript clipboard detection only works on macOS. Linux/Windows users fall back to arboard only (no file path detection from file managers).
- **Perceptual quantization changes all 256-color output**: Every image exported in 256-color mode will produce slightly different colors. This is intentional (better) but affects visual regression comparisons.
