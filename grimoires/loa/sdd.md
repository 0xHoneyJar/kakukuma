# SDD: Import Fidelity & Color Intelligence

> **Status**: Approved
> **Created**: 2026-03-06
> **Updated**: 2026-03-14 (expanded: clipboard import, perceptual quantization, UI improvements)
> **PRD**: grimoires/loa/prd.md
> **Cycle**: 022

---

## 1. Architecture Overview

This cycle modifies existing modules with one new file (`src/lib.rs`). Changes span smart defaults, a new CLI command, terminal detection, clipboard integration, perceptual color quantization, and TUI import UX improvements.

### Modified Files

| File | Changes |
|------|---------|
| `src/cli/mod.rs` | New `Render` command, default changes for `Import`/`Preview`/`Export`, char alias support |
| `src/export.rs` | `detect_terminal_colors()`, `ColorFormat::Auto`, hue-preserving export path |
| `src/import.rs` | Smart defaults, `import_image_data()` for clipboard, normalize/posterize pipeline, brightness lift, mosaic mode, full-blocks fix |
| `src/input.rs` | Clipboard import (Ctrl+V), paste buffer, import browse filter, import options toggles, keymap fixes |
| `src/ui/mod.rs` | Import browse filter UI, import options expansion, block picker info |
| `src/cell.rs` | Perceptual `nearest_256_hue()`, `CharInfo` metadata, `resolve_char_alias()` |
| `src/app.rs` | Clipboard/paste/filter state fields, command registry updates |
| `src/main.rs` | Bracketed paste enable, paste buffer tick |
| `src/ui/toolbar.rs` | Block panel 4-row redesign |
| `src/ui/statusbar.rs` | Keymap hints, spacebar mode indicator |
| `src/canvas.rs` | `is_empty()` method |

### New Files

| File | Purpose |
|------|---------|
| `src/lib.rs` | Library crate re-exports (from cycle-017, formalized) |

---

## 2. Design Decisions

### 2.1 Terminal Color Detection

```rust
// src/export.rs

/// Detect terminal color capabilities from environment.
pub fn detect_terminal_colors() -> ColorFormat {
    // 1. COLORTERM=truecolor or COLORTERM=24bit → TrueColor
    if let Ok(ct) = std::env::var("COLORTERM") {
        if ct == "truecolor" || ct == "24bit" {
            return ColorFormat::TrueColor;
        }
    }
    // 2. Fallback → Color256Hue (safe on all modern terminals)
    ColorFormat::Color256Hue
}
```

**Rationale**: `Color256Hue` is the safe default — it uses standard `\e[38;5;Nm` escapes that work everywhere, while preserving hue better than plain `Color256`. Truecolor is only used when we're confident the terminal supports it.

### 2.2 `ColorFormat::Auto` Variant

Add `Auto` to the `ColorFormat` enum. When `Auto` is selected:
- CLI commands call `detect_terminal_colors()` at render time
- This is the new default for `preview`, `export`, and `render`

```rust
pub enum ColorFormat {
    Auto,       // NEW — detect at render time
    TrueColor,
    Color256,
    Color256Hue,
    Color16,
}
```

Corresponding `CliColorFormat`:
```rust
pub enum CliColorFormat {
    Auto,          // NEW — default for preview/export/render
    Truecolor,
    Color256,
    Color256Hue,
    Color16,
}
```

### 2.3 Smart Import Defaults

Change `ImportOptions::default()`:

```rust
impl Default for ImportOptions {
    fn default() -> Self {
        ImportOptions {
            fit_mode: FitMode::FitToCanvas,
            color_mode: ImportColorMode::TrueColor,  // was Color256
            char_set: ImportCharSet::HalfBlocks,
            color_boost: 1.0,
            preserve_hue: true,   // was false
            normalize: true,      // was false
        }
    }
}
```

CLI `Import` command changes:
- `--quantize` default: `truecolor` (was `256`)
- `--normalize` default: always on, add `--no-normalize` to disable
- `--preserve-hue` default: always on, add `--no-preserve-hue` to disable

### 2.4 `Render` Command

```rust
/// Convert an image directly to ANSI art on stdout
Render {
    /// Path to image file (PNG, JPEG, etc.)
    image: String,
    /// Output width in characters
    #[arg(long, default_value_t = 48)]
    width: usize,
    /// Output height in cell rows
    #[arg(long, default_value_t = 24)]
    height: usize,
    /// Color format (auto-detected by default)
    #[arg(long, default_value = "auto")]
    color_format: CliColorFormat,
    /// Disable brightness normalization
    #[arg(long)]
    no_normalize: bool,
    /// Disable hue preservation
    #[arg(long)]
    no_preserve_hue: bool,
    /// Color saturation boost
    #[arg(long, default_value_t = 1.0)]
    boost: f32,
}
```

Implementation flow:
1. Open image, create `ImportOptions` with smart defaults
2. `import_image()` → get cells grid
3. Build temporary `Canvas` from cells
4. Resolve `ColorFormat::Auto` via `detect_terminal_colors()`
5. `to_ansi(&canvas, resolved_format)` → print to stdout

No intermediate file. No .kaku. Fastest path from image to terminal.

### 2.5 TUI Import Dialog Enhancement

Expand the import options dialog from 3 rows to 6:

| Row | Field | Key | Values |
|-----|-------|-----|--------|
| 0 | Fit mode | Left/Right | FitToCanvas, Custom |
| 1 | Color mode | Left/Right | TrueColor, 256, 16 |
| 2 | Char set | Left/Right | Full, Half |
| 3 | Normalize | N | ON/OFF |
| 4 | Hue preserve | H | ON/OFF |
| 5 | Posterize | Left/Right | Off, 8, 12, 16, 24 |

Auto color boost: 1.2× for 256-color, 1.4× for 16-color (applied automatically, not exposed in UI).

### 2.6 Clipboard Image Import

```rust
// src/input.rs
fn clipboard_import(app: &mut App) {
    // 1. Try osascript for file path (Finder Cmd+C)
    if let Some(path) = clipboard_file_path() {
        if path.is_file() && is_image_file(&path.to_string_lossy()) {
            app.import_path = Some(path);
            app.mode = AppMode::ImportOptions;
            return;
        }
    }
    // 2. Fall back to raw RGBA pixel data
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        if let Ok(img) = clipboard.get_image() {
            app.clipboard_image = Some((img.bytes.into(), img.width as u32, img.height as u32));
            app.mode = AppMode::ImportOptions;
        }
    }
}
```

macOS-specific: `osascript -e 'POSIX path of (the clipboard as «class furl»)'` reads file URLs from the pasteboard. Falls back to `arboard::get_image()` for screenshots.

### 2.7 Import Browse Type-to-Filter

State fields on `App`:
- `import_filter: String` — current filter query
- `import_all_entries: Vec<String>` — cached unfiltered directory listing

Behavior:
- Regular characters append to filter, re-filter displayed list
- `/` or `~` enters path mode (direct path entry with Tab completion)
- Backspace removes last char from filter
- Esc clears filter (if filter active) or closes dialog

### 2.8 Perceptual Color Quantization

Replace Euclidean RGB distance in `nearest_256_inner()` with "redmean" approximation:

```rust
let rmean = (src.r as f32 + candidate.r as f32) / 2.0;
let wr = 2.0 + rmean / 256.0;   // red weight varies with red level
let wg = 4.0;                     // green dominates (most perceptual weight)
let wb = 2.0 + (255.0 - rmean) / 256.0;  // blue inversely weighted
let dist = wr * dr² + wg * dg² + wb * db²;
```

Additional terms for hue-preserving mode:
- Gray penalty: 20000 (was 8000) when source has hue but candidate is gray
- Luminance preservation: `lum_diff² * 0.08 * dark_penalty` where dark_penalty=1.5 for darkening

Brightness lift in `boost_saturation()`: dark chromatic pixels (max channel < 150, saturation > 0.15) get scaled up to clear the dead zone, capped at 1.5× to prevent blowout.

### 2.9 Full Blocks Fix

`rasterize_full_blocks()` changed from:
```rust
Cell { ch: ' ', fg: None, bg: Some(rgb) }  // BROKEN: is_empty() returns true
```
to:
```rust
Cell { ch: '\u{2588}', fg: Some(rgb), bg: None }  // FIXED: full block char with fg
```

---

## 3. Data Flow

### `render` command (new)
```
image.png → import_image() → Vec<Vec<Cell>> → Canvas → to_ansi(Auto) → stdout
                                                           ↓
                                                  detect_terminal_colors()
```

### `import` command (modified defaults)
```
image.png → import_image(truecolor, normalize, preserve_hue) → Canvas → .kaku file
```

### `export`/`preview` commands (modified defaults)
```
.kaku file → Canvas → to_ansi(Auto) → detect_terminal_colors() → stdout/file
```

---

## 4. Testing Strategy

### Unit Tests

| Test | Location | What |
|------|----------|------|
| `test_detect_terminal_truecolor` | `src/export.rs` | Set COLORTERM=truecolor, verify TrueColor returned |
| `test_detect_terminal_256` | `src/export.rs` | Unset COLORTERM, verify Color256Hue fallback |
| `test_detect_terminal_fallback` | `src/export.rs` | No env vars, verify Color256Hue fallback |
| `test_auto_resolves` | `src/export.rs` | Verify Auto variant resolves to concrete format |
| `test_default_import_options` | `src/import.rs` | Verify new defaults: truecolor, normalize=true, preserve_hue=true |
| `test_render_produces_ansi` | `src/cli/mod.rs` | Render test image, verify output contains ANSI escapes |
| `test_no_normalize_flag` | `src/import.rs` | Verify --no-normalize disables normalization |
| `test_no_preserve_hue_flag` | `src/import.rs` | Verify --no-preserve-hue disables hue preservation |

### Integration Tests

- Import a test PNG with new defaults → export → verify output contains `\e[38;5;` escapes (not truecolor)
- `render` command with test image → verify non-empty stdout with ANSI codes

---

## 5. Migration & Compatibility

### Breaking Changes

| Change | Impact | Mitigation |
|--------|--------|------------|
| Import default `--quantize` now `truecolor` | Existing scripts using bare `kakukuma import` will store full RGB instead of pre-quantized 256 | .kaku files are always re-exported; stored color mode doesn't affect display |
| Import default normalize/preserve-hue now on | Results will look different (better for photos, potentially unwanted for pixel art) | `--no-normalize --no-preserve-hue` flags |
| Preview/export default color format now `auto` | Output may switch from truecolor to 256-color depending on terminal | Explicit `--color-format truecolor` override available |

### Backward Compatibility

- All existing CLI flags remain functional
- .kaku file format unchanged (version 5)
- TUI behavior unchanged except import dialog default toggles
