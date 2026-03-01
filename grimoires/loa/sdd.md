# SDD: CLI Polish & Image Export

> **Cycle**: 019
> **Created**: 2026-03-01
> **PRD**: grimoires/loa/prd.md

---

## 1. Executive Summary

Two focused changes to the kakukuma CLI: (1) normalize 4 commands that use required `--output`/`--commands` flags to positional arguments with backward-compatible aliases, and (2) add PNG image export via the existing `image` crate. No new files created — all changes are to existing modules. No TUI changes.

---

## 2. Architecture Overview

### Files Modified

| File | Changes |
|------|---------|
| `src/cli/mod.rs` | Refactor 4 Command variants (Export, Import, Batch, Reference) + PaletteAction::Export clap attributes. Add PNG format variant. Update help text for undo/clear. |
| `src/export.rs` | Add `to_png()` function with block character pixel rendering. Add `CellSize` struct. |
| `src/cli/preview.rs` | Route PNG format to new `to_png()` in export_to_file. |

### Files NOT Modified

- `src/app.rs` — TUI unchanged
- `src/input.rs` — TUI unchanged
- `src/ui/` — TUI unchanged
- `src/cell.rs` — Block constants unchanged
- `src/import.rs` — Import unchanged
- `src/project.rs` — Project format unchanged
- `src/oplog.rs` — Undo logic unchanged
- `src/history.rs` — Undo logic unchanged
- `Cargo.toml` — `image` crate already has PNG encode support

---

## 3. FR-1: CLI Argument Normalization

### 3.1 Current State

Four commands have required args behind `#[arg(long)]` flags:

```rust
// Export — current
Export {
    file: String,
    #[arg(long)]        // REQUIRED flag
    output: String,
    #[arg(long, default_value = "ansi")]
    format: PreviewFormat,
    #[arg(long, default_value = "truecolor")]
    color_format: CliColorFormat,
}

// Import — current
Import {
    image: String,
    #[arg(long)]        // REQUIRED flag
    output: String,
    #[arg(long, default_value_t = 48)]
    width: usize,
    #[arg(long, default_value_t = 32)]
    height: usize,
    #[arg(long, default_value = "256")]
    quantize: CliColorFormat,
}

// Batch — current
Batch {
    file: String,
    #[arg(long)]        // REQUIRED flag
    commands: String,
    #[arg(long)]
    dry_run: bool,
}

// PaletteAction::Export — current
Export {
    name: String,
    #[arg(long)]        // REQUIRED flag
    output: String,
}
```

### 3.2 Target State

Convert required flags to positional args. Use clap's `alias` to keep old `--output`/`--commands` working.

```rust
// Export — after
Export {
    /// Path to .kaku file
    file: String,
    /// Output file path
    output: String,                          // Now positional
    /// Export format
    #[arg(long, default_value = "auto")]     // Changed default to "auto"
    format: PreviewFormat,
    /// Color depth for ANSI output
    #[arg(long, default_value = "truecolor")]
    color_format: CliColorFormat,
    /// Cell size for PNG export (WxH pixels)
    #[arg(long, default_value = "8x16")]
    cell_size: String,
    /// Integer scale factor for PNG export
    #[arg(long, default_value_t = 1)]
    scale: u32,
    /// Export full canvas (skip auto-crop)
    #[arg(long)]
    no_crop: bool,
}

// Import — after
Import {
    /// Path to image file (PNG, JPEG, etc.)
    image: String,
    /// Path to output .kaku file
    output: String,                          // Now positional
    #[arg(long, default_value_t = 48)]
    width: usize,
    #[arg(long, default_value_t = 32)]
    height: usize,
    #[arg(long, default_value = "256")]
    quantize: CliColorFormat,
}

// Batch — after
Batch {
    /// Path to .kaku file
    file: String,
    /// Path to JSON commands file
    commands: String,                        // Now positional
    #[arg(long)]
    dry_run: bool,
}

// PaletteAction::Export — after
Export {
    name: String,
    /// Output file path
    output: String,                          // Now positional
}
```

### 3.3 Backward Compatibility Strategy

Clap does not natively support "positional OR flag" for the same field. Two approaches:

**Approach A (Recommended): Optional positional + optional flag, validate one is present**

```rust
Export {
    file: String,
    /// Output file path
    output: Option<String>,
    /// Output file path (deprecated, use positional)
    #[arg(long = "output", hide = true)]
    output_flag: Option<String>,
    // ...
}
```

Then in the dispatch:
```rust
let output = output.or(output_flag)
    .unwrap_or_else(|| cli_error("Output path required"));
```

**Approach B: Just make it positional, accept the breaking change**

Since the only consumers are agents (which adapt) and the tool is pre-1.0, simply change to positional. Document the change.

**Decision**: Use Approach A for `export` and `import` (most likely to have existing scripts). Use Approach B for `batch` and `palette export` (newer commands with fewer existing users).

### 3.4 Auto-Format Detection for Export

When `--format` is `"auto"` (new default), detect format from output file extension:

```rust
fn detect_format(output: &str, explicit: &PreviewFormat) -> PreviewFormat {
    if *explicit != PreviewFormat::Auto {
        return explicit.clone();
    }
    match Path::new(output).extension().and_then(|e| e.to_str()) {
        Some("png") => PreviewFormat::Png,
        Some("json") => PreviewFormat::Json,
        Some("txt") => PreviewFormat::Plain,
        _ => PreviewFormat::Ansi,  // Default fallback
    }
}
```

Add `Auto` and `Png` variants to `PreviewFormat` enum:
```rust
#[derive(Clone, ValueEnum)]
pub enum PreviewFormat {
    Auto,
    Ansi,
    Json,
    Plain,
    Png,
}
```

---

## 4. FR-2: PNG Image Export

### 4.1 Function Signature

New function in `src/export.rs`:

```rust
pub fn to_png(
    canvas: &Canvas,
    cell_w: u32,
    cell_h: u32,
    scale: u32,
    crop: bool,
) -> image::RgbaImage
```

### 4.2 Rendering Pipeline

```
Canvas cells → Bounding box (if crop) → Pixel grid → Block fill → Scale → RgbaImage
```

1. **Compute region**: If `crop`, use existing `bounding_box()`. Otherwise full canvas.
2. **Create image buffer**: `RgbaImage::new(region_w * cell_w, region_h * cell_h)`
3. **For each cell in region**: Call `render_cell_to_pixels()` to fill the cell's pixel block
4. **Scale**: If `scale > 1`, upscale with nearest-neighbor via `image::imageops::resize()` with `FilterType::Nearest`
5. **Return**: The `RgbaImage` (caller saves to disk)

### 4.3 Block Character Pixel Rendering

Each cell occupies a `cell_w × cell_h` pixel block. The rendering depends on the character:

```rust
fn render_cell_to_pixels(
    img: &mut RgbaImage,
    cell: &Cell,
    px: u32,    // pixel x origin
    py: u32,    // pixel y origin
    cw: u32,    // cell width in pixels
    ch: u32,    // cell height in pixels
)
```

**Rendering rules by character type:**

| Character | Pixel Fill Rule |
|-----------|----------------|
| `█` (FULL) | Fill entire block with fg color |
| `▀` (UPPER_HALF) | Top half fg, bottom half bg (or transparent) |
| `▄` (LOWER_HALF) | Bottom half fg, top half bg (or transparent) |
| `▌` (LEFT_HALF) | Left half fg, right half bg (or transparent) |
| `▐` (RIGHT_HALF) | Right half fg, left half bg (or transparent) |
| `░` (SHADE_LIGHT) | 25% fg pixels, 75% bg pixels (dither pattern) |
| `▒` (SHADE_MEDIUM) | 50% fg pixels, 50% bg pixels (checkerboard) |
| `▓` (SHADE_DARK) | 75% fg pixels, 25% bg pixels (inverse dither) |
| `▁` (LOWER_1_8) | Bottom 1/8 fg, rest bg |
| `▂` (LOWER_1_4) | Bottom 1/4 fg, rest bg |
| `▃` (LOWER_3_8) | Bottom 3/8 fg, rest bg |
| `▅` (LOWER_5_8) | Bottom 5/8 fg, rest bg |
| `▆` (LOWER_3_4) | Bottom 3/4 fg, rest bg |
| `▇` (LOWER_7_8) | Bottom 7/8 fg, rest bg |
| `▉` (LEFT_7_8) | Left 7/8 fg, rest bg |
| `▊` (LEFT_3_4) | Left 3/4 fg, rest bg |
| `▋` (LEFT_5_8) | Left 5/8 fg, rest bg |
| `▍` (LEFT_3_8) | Left 3/8 fg, rest bg |
| `▎` (LEFT_1_4) | Left 1/4 fg, rest bg |
| `▏` (LEFT_1_8) | Left 1/8 fg, rest bg |
| ` ` (space/empty) | Fill with bg color, or transparent if no bg |
| Any other char | Fill entire block with fg color (simplified) |

**Color mapping:**
- `fg: Some(Rgb)` → `Rgba([r, g, b, 255])` — use raw RGB, NOT `to_ratatui()`
- `bg: Some(Rgb)` → `Rgba([r, g, b, 255])`
- `fg: None` or `bg: None` → `Rgba([0, 0, 0, 0])` (fully transparent)

**Shade dither pattern:**
```rust
fn shade_pixel(x: u32, y: u32, shade: char) -> bool {
    // Returns true if this pixel should be fg color
    let pattern = (x + y) % 4;  // 4x4 tile
    match shade {
        SHADE_LIGHT  => pattern == 0,                    // 25% fill
        SHADE_MEDIUM => (x + y) % 2 == 0,               // 50% checkerboard
        SHADE_DARK   => pattern != 0,                    // 75% fill
        _ => false,
    }
}
```

### 4.4 Fractional Fill Implementation

For vertical fractional fills (LOWER_1_8 through LOWER_7_8):
```rust
fn vertical_fraction(ch: char) -> f32 {
    match ch {
        LOWER_1_8 => 1.0 / 8.0,
        LOWER_1_4 => 2.0 / 8.0,
        LOWER_3_8 => 3.0 / 8.0,
        LOWER_HALF => 4.0 / 8.0,  // Included for completeness
        LOWER_5_8 => 5.0 / 8.0,
        LOWER_3_4 => 6.0 / 8.0,
        LOWER_7_8 => 7.0 / 8.0,
        _ => 0.0,
    }
}
// Pixel (x, y_in_cell) is fg if y_in_cell >= cell_h * (1.0 - fraction)
```

For horizontal fractional fills (LEFT_1_8 through LEFT_7_8):
```rust
fn horizontal_fraction(ch: char) -> f32 {
    match ch {
        LEFT_1_8 => 1.0 / 8.0,
        LEFT_1_4 => 2.0 / 8.0,
        LEFT_3_8 => 3.0 / 8.0,
        LEFT_HALF => 4.0 / 8.0,
        LEFT_5_8 => 5.0 / 8.0,
        LEFT_3_4 => 6.0 / 8.0,
        LEFT_7_8 => 7.0 / 8.0,
        _ => 0.0,
    }
}
// Pixel (x_in_cell, y) is fg if x_in_cell < cell_w * fraction
```

### 4.5 CLI Integration

In `src/cli/preview.rs`, the `export_to_file()` function routes to `to_png()`:

```rust
PreviewFormat::Png => {
    let (cw, ch) = parse_cell_size(&cell_size)?;  // "8x16" → (8, 16)
    let img = export::to_png(&project.canvas, cw, ch, scale, !no_crop);
    img.save(&output).map_err(|e| {
        io::Error::new(io::ErrorKind::Other, format!("PNG save failed: {}", e))
    })?;
    let json = serde_json::json!({
        "exported": output,
        "format": "png",
        "width": img.width(),
        "height": img.height(),
        "cell_size": format!("{}x{}", cw, ch),
    });
    println!("{}", serde_json::to_string(&json).unwrap());
}
```

### 4.6 Cell Size Parsing

```rust
fn parse_cell_size(s: &str) -> Result<(u32, u32), String> {
    let parts: Vec<&str> = s.split('x').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid cell size '{}', expected WxH (e.g., 8x16)", s));
    }
    let w = parts[0].parse::<u32>().map_err(|_| format!("Invalid width in '{}'", s))?;
    let h = parts[1].parse::<u32>().map_err(|_| format!("Invalid height in '{}'", s))?;
    if w == 0 || h == 0 || w > 64 || h > 64 {
        return Err(format!("Cell size {}x{} out of range (1-64)", w, h));
    }
    Ok((w, h))
}
```

---

## 5. FR-3: Undo Documentation

### 5.1 Changes to Help Text

In `src/cli/mod.rs`, update the doc comments for `Undo` and `Clear`:

```rust
/// Undo last CLI operation.
///
/// Undo uses a linear model: new operations discard redo history.
/// Operations that overlap (e.g., clear over a drawn rect) store
/// only the cleared state — undoing the clear restores the clear's
/// snapshot, not the original drawn content.
Undo {
    file: String,
    #[arg(long, default_value_t = 1)]
    count: usize,
}

/// Clear canvas (reset all cells to default).
///
/// Warning: clear is destructive. If clear overlaps with prior
/// operations, undoing the clear may not fully restore all
/// previous content. Consider exporting a backup first.
Clear {
    file: String,
    #[arg(long)]
    region: Option<String>,
}
```

---

## 6. Testing Strategy

### 6.1 CLI Normalization Tests

| Test | Description |
|------|-------------|
| `test_export_positional_output` | `export FILE OUTPUT --format plain` works |
| `test_export_flag_backward_compat` | `export FILE --output OUTPUT --format plain` still works |
| `test_import_positional_output` | `import IMAGE OUTPUT` works |
| `test_import_flag_backward_compat` | `import IMAGE --output OUTPUT` still works |
| `test_batch_positional_commands` | `batch FILE COMMANDS` works |
| `test_auto_format_png` | `export FILE out.png` auto-detects PNG format |
| `test_auto_format_txt` | `export FILE out.txt` auto-detects plain format |
| `test_auto_format_default_ansi` | `export FILE out.ans` falls back to ANSI |

### 6.2 PNG Export Tests

| Test | Description |
|------|-------------|
| `test_png_empty_canvas` | Empty canvas produces transparent PNG |
| `test_png_single_cell` | One filled cell produces correct pixel block |
| `test_png_full_block` | `█` fills entire cell with fg color |
| `test_png_upper_half` | `▀` fills top half fg, bottom half bg/transparent |
| `test_png_lower_half` | `▄` fills bottom half fg, top half bg/transparent |
| `test_png_left_half` | `▌` fills left half fg, right half bg/transparent |
| `test_png_right_half` | `▐` fills right half fg, left half bg/transparent |
| `test_png_shade_light` | `░` produces ~25% fg pixel density |
| `test_png_shade_medium` | `▒` produces ~50% fg pixel density |
| `test_png_shade_dark` | `▓` produces ~75% fg pixel density |
| `test_png_vertical_fraction` | `▂` (1/4) fills bottom quarter with fg |
| `test_png_horizontal_fraction` | `▊` (3/4) fills left three-quarters with fg |
| `test_png_auto_crop` | Only bounding box region exported (default) |
| `test_png_no_crop` | Full canvas exported with `--no-crop` |
| `test_png_scale_2x` | Scaled output is 2x dimensions, nearest-neighbor |
| `test_png_transparent_bg` | Cells with `bg: None` have alpha=0 pixels |
| `test_png_cell_size_custom` | Custom `--cell-size 4x8` produces smaller pixels |

### 6.3 Regression

All 356 existing tests must continue passing.

---

## 7. Performance Considerations

- **PNG generation**: 128×128 canvas at 8×16 = 1024×2048 pixels = ~8MB uncompressed RGBA. PNG compression brings this down to ~100KB-1MB. Target: <500ms.
- **Scale factor**: `--scale 4` on max canvas = 4096×8192 pixels. This is large but still <100ms with simple pixel copy. Cap scale at 8 to prevent accidental multi-GB images.
- **No font rendering**: Block character rendering is pure geometry (rectangle fills), not glyph rasterization. This is fast and dependency-free.

---

## 8. Security Considerations

- **Path traversal**: Output path for export is user-provided. No special handling needed — `image::save()` writes to the given path. No network access.
- **Memory**: Max image size (128×128 canvas × 8×16 cell × 8 scale) = 8192×16384 × 4 bytes = ~512MB. Cap scale at 8 and validate cell size (max 64×64) to prevent OOM.
- **Input validation**: Cell size parsing rejects 0 or >64. Scale rejects 0 or >8.
