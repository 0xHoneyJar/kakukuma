use image::{Rgba, RgbaImage};

use crate::canvas::Canvas;
use crate::cell::{blocks, is_half_block, nearest_16, nearest_256, resolve_half_block, ResolvedHalfBlock, Rgb};

/// ANSI color format for export.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ColorFormat {
    /// 24-bit true color: \x1b[38;2;R;G;Bm
    TrueColor,
    /// xterm 256-color: \x1b[38;5;Nm
    Color256,
    /// ANSI 16-color: \x1b[38;5;Nm (N in 0–15)
    Color16,
}

/// Returns the bounding box of all non-empty cells as (min_x, min_y, max_x, max_y),
/// or None if the canvas is entirely empty.
pub fn bounding_box(canvas: &Canvas) -> Option<(usize, usize, usize, usize)> {
    let mut min_x = canvas.width;
    let mut min_y = canvas.height;
    let mut max_x = 0usize;
    let mut max_y = 0usize;

    for y in 0..canvas.height {
        for x in 0..canvas.width {
            if let Some(cell) = canvas.get(x, y) {
                if !cell.is_empty() {
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                }
            }
        }
    }

    if max_x >= min_x && max_y >= min_y {
        Some((min_x, min_y, max_x, max_y))
    } else {
        None
    }
}

/// Export canvas as plain Unicode (block characters only, no color).
/// Auto-crops to bounding box.
pub fn to_plain_text(canvas: &Canvas) -> String {
    let (min_x, min_y, max_x, max_y) = match bounding_box(canvas) {
        Some(bb) => bb,
        None => return String::new(),
    };

    let mut output = String::new();
    for y in min_y..=max_y {
        let mut row = String::new();
        for x in min_x..=max_x {
            if let Some(cell) = canvas.get(x, y) {
                row.push(cell.ch);
            }
        }
        // Strip trailing spaces
        let trimmed = row.trim_end();
        output.push_str(trimmed);
        if y < max_y {
            output.push('\n');
        }
    }

    output
}

/// Emit ANSI fg escape code for a color in the given format.
fn emit_fg(color: &Rgb, format: ColorFormat) -> String {
    match format {
        ColorFormat::TrueColor => format!("\x1b[38;2;{};{};{}m", color.r, color.g, color.b),
        ColorFormat::Color256 => format!("\x1b[38;5;{}m", nearest_256(color)),
        ColorFormat::Color16 => format!("\x1b[38;5;{}m", nearest_16(color)),
    }
}

/// Emit ANSI fg+bg escape code for colors in the given format.
fn emit_fg_bg(fg: &Rgb, bg: &Rgb, format: ColorFormat) -> String {
    match format {
        ColorFormat::TrueColor => format!(
            "\x1b[38;2;{};{};{};48;2;{};{};{}m",
            fg.r, fg.g, fg.b, bg.r, bg.g, bg.b
        ),
        ColorFormat::Color256 => format!(
            "\x1b[38;5;{};48;5;{}m",
            nearest_256(fg), nearest_256(bg)
        ),
        ColorFormat::Color16 => format!(
            "\x1b[38;5;{};48;5;{}m",
            nearest_16(fg), nearest_16(bg)
        ),
    }
}

/// Emit ANSI bg escape code for a color in the given format.
fn emit_bg(color: &Rgb, format: ColorFormat) -> String {
    match format {
        ColorFormat::TrueColor => format!("\x1b[48;2;{};{};{}m", color.r, color.g, color.b),
        ColorFormat::Color256 => format!("\x1b[48;5;{}m", nearest_256(color)),
        ColorFormat::Color16 => format!("\x1b[48;5;{}m", nearest_16(color)),
    }
}

/// Emit color escape codes, tracking previous values to avoid redundant output.
fn emit_cell_colors(
    output: &mut String,
    fg: Option<Rgb>,
    bg: Option<Rgb>,
    prev_fg: &mut Option<Rgb>,
    prev_bg: &mut Option<Rgb>,
    format: ColorFormat,
) {
    let fg_changed = *prev_fg != fg;
    let bg_changed = *prev_bg != bg;

    if !fg_changed && !bg_changed {
        return;
    }

    match (fg, bg) {
        (Some(f), Some(b)) => {
            output.push_str(&emit_fg_bg(&f, &b, format));
        }
        (Some(f), None) => {
            output.push_str(&emit_fg(&f, format));
            if bg_changed && prev_bg.is_some() {
                output.push_str("\x1b[49m"); // reset bg
            }
        }
        (None, Some(b)) => {
            output.push_str(&emit_bg(&b, format));
            if fg_changed && prev_fg.is_some() {
                output.push_str("\x1b[39m"); // reset fg
            }
        }
        (None, None) => {
            output.push_str("\x1b[0m");
        }
    }

    *prev_fg = fg;
    *prev_bg = bg;
}

/// Export canvas as ANSI art (Unicode blocks with color escape codes).
/// Auto-crops to bounding box. Applies half-block resolution for export fidelity.
/// Color format determines escape sequence type (24-bit, 256-color, or 16-color).
pub fn to_ansi(canvas: &Canvas, format: ColorFormat) -> String {
    let (min_x, min_y, max_x, max_y) = match bounding_box(canvas) {
        Some(bb) => bb,
        None => return String::new(),
    };

    let mut output = String::new();

    for y in min_y..=max_y {
        let mut prev_fg: Option<Rgb> = None;
        let mut prev_bg: Option<Rgb> = None;

        for x in min_x..=max_x {
            if let Some(cell) = canvas.get(x, y) {
                if cell.is_empty() {
                    output.push(' ');
                    continue;
                }

                // Determine effective (ch, fg, bg) — half-block resolution or raw cell
                let (out_ch, fg, bg) = if is_half_block(cell.ch) {
                    let resolved = resolve_half_block(&cell).unwrap_or(ResolvedHalfBlock {
                        ch: cell.ch, fg: cell.fg, bg: cell.bg,
                    });
                    (resolved.ch, resolved.fg, resolved.bg)
                } else {
                    (cell.ch, cell.fg, cell.bg)
                };

                if out_ch == ' ' {
                    // Both halves transparent after resolution
                    output.push(' ');
                    prev_fg = None;
                    prev_bg = None;
                    continue;
                }

                emit_cell_colors(&mut output, fg, bg, &mut prev_fg, &mut prev_bg, format);
                output.push(out_ch);
            }
        }

        output.push_str("\x1b[0m"); // Reset at end of line
        if y < max_y {
            output.push('\n');
        }
    }

    output
}

// --- PNG Export ---

/// Convert an Rgb color to an opaque RGBA pixel.
fn rgb_to_rgba(c: &Rgb) -> Rgba<u8> {
    Rgba([c.r, c.g, c.b, 255])
}

/// Transparent pixel (alpha = 0).
const TRANSPARENT: Rgba<u8> = Rgba([0, 0, 0, 0]);

/// Returns true if this pixel should use fg color for shade characters.
fn shade_pixel(x: u32, y: u32, shade: char) -> bool {
    match shade {
        blocks::SHADE_LIGHT => (x + y) % 4 == 0,          // 25%
        blocks::SHADE_MEDIUM => (x + y) % 2 == 0,         // 50% checkerboard
        blocks::SHADE_DARK => (x + y) % 4 != 0,           // 75%
        _ => false,
    }
}

/// Vertical fractional fill amount (fills from bottom).
fn vertical_fraction(ch: char) -> Option<f32> {
    match ch {
        blocks::LOWER_1_8 => Some(1.0 / 8.0),
        blocks::LOWER_1_4 => Some(2.0 / 8.0),
        blocks::LOWER_3_8 => Some(3.0 / 8.0),
        blocks::LOWER_HALF => Some(4.0 / 8.0),
        blocks::LOWER_5_8 => Some(5.0 / 8.0),
        blocks::LOWER_3_4 => Some(6.0 / 8.0),
        blocks::LOWER_7_8 => Some(7.0 / 8.0),
        _ => None,
    }
}

/// Horizontal fractional fill amount (fills from left).
fn horizontal_fraction(ch: char) -> Option<f32> {
    match ch {
        blocks::LEFT_1_8 => Some(1.0 / 8.0),
        blocks::LEFT_1_4 => Some(2.0 / 8.0),
        blocks::LEFT_3_8 => Some(3.0 / 8.0),
        blocks::LEFT_HALF => Some(4.0 / 8.0),
        blocks::LEFT_5_8 => Some(5.0 / 8.0),
        blocks::LEFT_3_4 => Some(6.0 / 8.0),
        blocks::LEFT_7_8 => Some(7.0 / 8.0),
        _ => None,
    }
}

/// Render a single cell's pixels into the image buffer.
fn render_cell_to_pixels(
    img: &mut RgbaImage,
    cell: &crate::cell::Cell,
    px: u32,
    py: u32,
    cw: u32,
    ch_h: u32,
) {
    let fg_rgba = cell.fg.as_ref().map(rgb_to_rgba).unwrap_or(TRANSPARENT);
    let bg_rgba = cell.bg.as_ref().map(rgb_to_rgba).unwrap_or(TRANSPARENT);
    let ch = cell.ch;

    if ch == blocks::FULL {
        // Full block: entire cell is fg
        fill_rect(img, px, py, cw, ch_h, fg_rgba);
        return;
    }

    if ch == blocks::UPPER_HALF {
        let half = ch_h / 2;
        fill_rect(img, px, py, cw, half, fg_rgba);
        fill_rect(img, px, py + half, cw, ch_h - half, bg_rgba);
        return;
    }

    if ch == blocks::LOWER_HALF {
        let half = ch_h / 2;
        fill_rect(img, px, py, cw, half, bg_rgba);
        fill_rect(img, px, py + half, cw, ch_h - half, fg_rgba);
        return;
    }

    if ch == blocks::LEFT_HALF {
        let half = cw / 2;
        fill_rect(img, px, py, half, ch_h, fg_rgba);
        fill_rect(img, px + half, py, cw - half, ch_h, bg_rgba);
        return;
    }

    if ch == blocks::RIGHT_HALF {
        let half = cw / 2;
        fill_rect(img, px, py, half, ch_h, bg_rgba);
        fill_rect(img, px + half, py, cw - half, ch_h, fg_rgba);
        return;
    }

    // Shade characters
    if ch == blocks::SHADE_LIGHT || ch == blocks::SHADE_MEDIUM || ch == blocks::SHADE_DARK {
        for dy in 0..ch_h {
            for dx in 0..cw {
                let pixel = if shade_pixel(dx, dy, ch) { fg_rgba } else { bg_rgba };
                img.put_pixel(px + dx, py + dy, pixel);
            }
        }
        return;
    }

    // Vertical fractional fills (from bottom)
    if let Some(frac) = vertical_fraction(ch) {
        let fg_rows = (ch_h as f32 * frac).round() as u32;
        let bg_rows = ch_h - fg_rows;
        fill_rect(img, px, py, cw, bg_rows, bg_rgba);
        fill_rect(img, px, py + bg_rows, cw, fg_rows, fg_rgba);
        return;
    }

    // Horizontal fractional fills (from left)
    if let Some(frac) = horizontal_fraction(ch) {
        let fg_cols = (cw as f32 * frac).round() as u32;
        let bg_cols = cw - fg_cols;
        fill_rect(img, px, py, fg_cols, ch_h, fg_rgba);
        fill_rect(img, px + fg_cols, py, bg_cols, ch_h, bg_rgba);
        return;
    }

    // Space or empty
    if ch == ' ' {
        fill_rect(img, px, py, cw, ch_h, bg_rgba);
        return;
    }

    // Any other printable character: fill with fg
    fill_rect(img, px, py, cw, ch_h, fg_rgba);
}

/// Fill a rectangular region with a single color.
fn fill_rect(img: &mut RgbaImage, x: u32, y: u32, w: u32, h: u32, color: Rgba<u8>) {
    for dy in 0..h {
        for dx in 0..w {
            img.put_pixel(x + dx, y + dy, color);
        }
    }
}

/// Export canvas as a PNG image.
///
/// Each canvas cell maps to a `cell_w × cell_h` pixel block.
/// If `crop` is true, only the bounding box of non-empty cells is exported.
/// If `scale > 1`, the image is upscaled with nearest-neighbor interpolation.
pub fn to_png(
    canvas: &Canvas,
    cell_w: u32,
    cell_h: u32,
    scale: u32,
    crop: bool,
) -> RgbaImage {
    let scale = scale.clamp(1, 8);

    let (min_x, min_y, max_x, max_y) = if crop {
        match bounding_box(canvas) {
            Some(bb) => bb,
            None => return RgbaImage::new(1, 1), // Empty canvas
        }
    } else {
        (0, 0, canvas.width.saturating_sub(1), canvas.height.saturating_sub(1))
    };

    let region_w = (max_x - min_x + 1) as u32;
    let region_h = (max_y - min_y + 1) as u32;
    let img_w = region_w * cell_w;
    let img_h = region_h * cell_h;

    let mut img = RgbaImage::new(img_w, img_h);

    for cy in min_y..=max_y {
        for cx in min_x..=max_x {
            if let Some(cell) = canvas.get(cx, cy) {
                let px = (cx - min_x) as u32 * cell_w;
                let py = (cy - min_y) as u32 * cell_h;
                render_cell_to_pixels(&mut img, &cell, px, py, cell_w, cell_h);
            }
        }
    }

    if scale > 1 {
        let new_w = img_w * scale;
        let new_h = img_h * scale;
        image::imageops::resize(&img, new_w, new_h, image::imageops::FilterType::Nearest)
    } else {
        img
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::{blocks, Cell, Rgb, color256_to_rgb};

    const RED: Option<Rgb> = Some(Rgb { r: 205, g: 0, b: 0 });

    #[test]
    fn test_plain_text_empty() {
        let canvas = Canvas::new();
        let text = to_plain_text(&canvas);
        assert!(text.is_empty());
    }

    #[test]
    fn test_plain_text_single_block() {
        let mut canvas = Canvas::new();
        canvas.set(0, 0, Cell {
            ch: blocks::FULL,
            fg: RED,
            bg: None,
        });
        let text = to_plain_text(&canvas);
        assert_eq!(text, "\u{2588}");
    }

    #[test]
    fn test_plain_text_no_gaps() {
        let mut canvas = Canvas::new();
        for x in 0..3 {
            canvas.set(x, 0, Cell {
                ch: blocks::FULL,
                fg: Some(Rgb::WHITE),
                bg: None,
            });
        }
        let text = to_plain_text(&canvas);
        assert_eq!(text, "\u{2588}\u{2588}\u{2588}");
        assert!(!text.contains(' '));
    }

    #[test]
    fn test_ansi_256_color_codes() {
        let mut canvas = Canvas::new();
        canvas.set(0, 0, Cell {
            ch: blocks::FULL,
            fg: RED,
            bg: None,
        });
        let ansi = to_ansi(&canvas, ColorFormat::Color256);
        // Red (205,0,0) should quantize to index 1
        assert!(ansi.contains("\x1b[38;5;1m"));
        assert!(ansi.contains("\x1b[0m"));
    }

    #[test]
    fn test_ansi_truecolor() {
        let mut canvas = Canvas::new();
        canvas.set(0, 0, Cell {
            ch: blocks::FULL,
            fg: Some(Rgb::new(255, 0, 0)),
            bg: None,
        });
        let ansi = to_ansi(&canvas, ColorFormat::TrueColor);
        assert!(ansi.contains("\x1b[38;2;255;0;0m"));
    }

    #[test]
    fn test_ansi_16_color() {
        let mut canvas = Canvas::new();
        canvas.set(0, 0, Cell {
            ch: blocks::FULL,
            fg: Some(Rgb::new(255, 0, 0)),
            bg: None,
        });
        let ansi = to_ansi(&canvas, ColorFormat::Color16);
        // Pure red should quantize to ANSI 16-color index 9 (bright red)
        assert!(ansi.contains("38;5;"));
        assert!(ansi.contains("\x1b[0m"));
    }

    #[test]
    fn test_ansi_with_bg_color() {
        let mut canvas = Canvas::new();
        canvas.set(0, 0, Cell {
            ch: blocks::FULL,
            fg: Some(color256_to_rgb(7)),
            bg: Some(color256_to_rgb(4)),
        });
        let ansi = to_ansi(&canvas, ColorFormat::Color256);
        assert!(ansi.contains("\x1b[38;5;7;48;5;4m"));
    }

    // --- Bounding box tests ---

    #[test]
    fn test_bounding_box_empty_canvas() {
        let canvas = Canvas::new();
        assert_eq!(bounding_box(&canvas), None);
    }

    #[test]
    fn test_bounding_box_single_cell() {
        let mut canvas = Canvas::new();
        canvas.set(5, 3, Cell {
            ch: blocks::FULL,
            fg: RED,
            bg: None,
        });
        assert_eq!(bounding_box(&canvas), Some((5, 3, 5, 3)));
    }

    #[test]
    fn test_bounding_box_corner_art() {
        let mut canvas = Canvas::new_with_size(64, 64);
        for x in 10..=12 {
            for y in 10..=11 {
                canvas.set(x, y, Cell {
                    ch: blocks::FULL,
                    fg: RED,
                    bg: None,
                });
            }
        }
        assert_eq!(bounding_box(&canvas), Some((10, 10, 12, 11)));
    }

    #[test]
    fn test_plain_text_autocrop() {
        let mut canvas = Canvas::new();
        canvas.set(5, 3, Cell {
            ch: blocks::FULL,
            fg: RED,
            bg: None,
        });
        let text = to_plain_text(&canvas);
        assert_eq!(text, "\u{2588}");
        assert!(!text.starts_with('\n'));
        assert!(!text.starts_with(' '));
    }

    #[test]
    fn test_ansi_autocrop() {
        let mut canvas = Canvas::new();
        canvas.set(5, 3, Cell {
            ch: blocks::FULL,
            fg: RED,
            bg: None,
        });
        let ansi = to_ansi(&canvas, ColorFormat::Color256);
        assert!(ansi.starts_with("\x1b["));
        assert!(!ansi.contains('\n'));
    }

    #[test]
    fn test_nearest_16_basic() {
        // Pure white should map to index 15 (bright white)
        let white = Rgb::new(255, 255, 255);
        assert_eq!(nearest_16(&white), 15);
        // Pure black should map to index 0
        let black = Rgb::new(0, 0, 0);
        assert_eq!(nearest_16(&black), 0);
    }

    #[test]
    fn test_truecolor_fg_bg() {
        let mut canvas = Canvas::new();
        canvas.set(0, 0, Cell {
            ch: blocks::FULL,
            fg: Some(Rgb::new(100, 200, 50)),
            bg: Some(Rgb::new(10, 20, 30)),
        });
        let ansi = to_ansi(&canvas, ColorFormat::TrueColor);
        assert!(ansi.contains("\x1b[38;2;100;200;50;48;2;10;20;30m"));
    }

    // --- Half-block export fidelity tests ---

    #[test]
    fn test_export_halfblock_one_transparent_flips() {
        // UPPER_HALF with transparent top (fg=None) and opaque bottom (bg=BLUE)
        // Should flip to LOWER_HALF with fg=BLUE
        let mut canvas = Canvas::new();
        let blue = Rgb::new(0, 0, 238);
        canvas.set(0, 0, Cell {
            ch: blocks::UPPER_HALF,
            fg: None,
            bg: Some(blue),
        });
        let ansi = to_ansi(&canvas, ColorFormat::Color256);
        // Should contain LOWER_HALF character (▄) not UPPER_HALF (▀)
        assert!(ansi.contains('▄'), "Expected flipped char ▄, got: {}", ansi);
        assert!(!ansi.contains('▀'), "Should not contain original ▀");
        // Should have fg color for blue (index 4)
        assert!(ansi.contains("\x1b[38;5;4m"), "Expected fg blue: {}", ansi);
    }

    #[test]
    fn test_export_halfblock_both_transparent_is_space() {
        // UPPER_HALF with both halves transparent → space, no color codes
        let mut canvas = Canvas::new();
        canvas.set(0, 0, Cell {
            ch: blocks::UPPER_HALF,
            fg: None,
            bg: None,
        });
        // This cell is not "empty" (ch != ' '), but after resolution becomes space
        // However, bounding_box checks is_empty() which checks ch == ' ', so this cell
        // IS considered non-empty for bounding box. Let's add a real cell too.
        canvas.set(1, 0, Cell {
            ch: blocks::FULL,
            fg: RED,
            bg: None,
        });
        let ansi = to_ansi(&canvas, ColorFormat::Color256);
        // First cell should be a space (resolved from both-transparent half-block)
        // The output starts with a space before the color code for the FULL block
        assert!(ansi.starts_with(' '), "Expected space at start: {}", ansi);
    }

    #[test]
    fn test_export_black_bg_emits_color_code() {
        // Intentional black background should emit bg color code (not skipped)
        let mut canvas = Canvas::new();
        let white = Rgb::new(229, 229, 229);
        let black = Rgb::new(0, 0, 0);
        canvas.set(0, 0, Cell {
            ch: blocks::UPPER_HALF,
            fg: Some(white),
            bg: Some(black),
        });
        let ansi = to_ansi(&canvas, ColorFormat::Color256);
        // Should contain both fg and bg codes (fg+bg combined)
        assert!(ansi.contains(";48;5;"), "Expected bg code for black: {}", ansi);
    }

    #[test]
    fn test_export_left_half_transparent_left_flips() {
        // LEFT_HALF with transparent left (fg=None) and opaque right (bg=RED)
        // Should flip to RIGHT_HALF with fg=RED
        let mut canvas = Canvas::new();
        let red = Rgb { r: 205, g: 0, b: 0 };
        canvas.set(0, 0, Cell {
            ch: blocks::LEFT_HALF,
            fg: None,
            bg: Some(red),
        });
        let ansi = to_ansi(&canvas, ColorFormat::Color256);
        assert!(ansi.contains('▐'), "Expected flipped char ▐, got: {}", ansi);
        assert!(!ansi.contains('▌'), "Should not contain original ▌");
    }

    #[test]
    fn test_export_halfblock_both_opaque() {
        // UPPER_HALF with both halves opaque → normal char with fg+bg
        let mut canvas = Canvas::new();
        let red = Rgb { r: 205, g: 0, b: 0 };
        let blue = Rgb::new(0, 0, 238);
        canvas.set(0, 0, Cell {
            ch: blocks::UPPER_HALF,
            fg: Some(red),
            bg: Some(blue),
        });
        let ansi = to_ansi(&canvas, ColorFormat::Color256);
        assert!(ansi.contains('▀'), "Expected ▀ for both opaque");
        assert!(ansi.contains("\x1b[38;5;1;48;5;4m"), "Expected fg+bg: {}", ansi);
    }

    // --- Shade character export tests (Cycle 15 QA) ---

    #[test]
    fn test_export_shade_light_fg_only() {
        let mut canvas = Canvas::new();
        canvas.set(0, 0, Cell {
            ch: blocks::SHADE_LIGHT,
            fg: RED,
            bg: None,
        });
        let ansi = to_ansi(&canvas, ColorFormat::Color256);
        assert!(ansi.contains('░'), "Expected ░ in output: {}", ansi);
        assert!(ansi.contains("\x1b[38;5;1m"), "Expected fg-only code: {}", ansi);
    }

    #[test]
    fn test_export_shade_medium_fg_only() {
        let mut canvas = Canvas::new();
        let green = Some(Rgb::new(0, 205, 0));
        canvas.set(0, 0, Cell {
            ch: blocks::SHADE_MEDIUM,
            fg: green,
            bg: None,
        });
        let ansi = to_ansi(&canvas, ColorFormat::Color256);
        assert!(ansi.contains('▒'), "Expected ▒ in output: {}", ansi);
        assert!(ansi.contains("\x1b[38;5;"), "Expected fg code: {}", ansi);
    }

    #[test]
    fn test_export_shade_dark_fg_only() {
        let mut canvas = Canvas::new();
        let blue = Some(Rgb::new(0, 0, 238));
        canvas.set(0, 0, Cell {
            ch: blocks::SHADE_DARK,
            fg: blue,
            bg: None,
        });
        let ansi = to_ansi(&canvas, ColorFormat::Color256);
        assert!(ansi.contains('▓'), "Expected ▓ in output: {}", ansi);
        assert!(ansi.contains("\x1b[38;5;"), "Expected fg code: {}", ansi);
    }

    #[test]
    fn test_export_shade_with_bg() {
        let mut canvas = Canvas::new();
        let white = Rgb::new(229, 229, 229);
        let black = Rgb::new(0, 0, 0);
        canvas.set(0, 0, Cell {
            ch: blocks::SHADE_MEDIUM,
            fg: Some(white),
            bg: Some(black),
        });
        let ansi = to_ansi(&canvas, ColorFormat::Color256);
        assert!(ansi.contains('▒'), "Expected ▒");
        // Should have both fg and bg codes
        assert!(ansi.contains(";48;5;"), "Expected bg code: {}", ansi);
        assert!(ansi.contains("38;5;"), "Expected fg code: {}", ansi);
    }

    #[test]
    fn test_export_shade_256_color() {
        let mut canvas = Canvas::new();
        canvas.set(0, 0, Cell {
            ch: blocks::SHADE_LIGHT,
            fg: RED,
            bg: None,
        });
        let ansi = to_ansi(&canvas, ColorFormat::Color256);
        assert!(ansi.contains("\x1b[38;5;"), "256-color fg code: {}", ansi);
    }

    #[test]
    fn test_export_shade_16_color() {
        let mut canvas = Canvas::new();
        canvas.set(0, 0, Cell {
            ch: blocks::SHADE_LIGHT,
            fg: RED,
            bg: None,
        });
        let ansi = to_ansi(&canvas, ColorFormat::Color16);
        assert!(ansi.contains("\x1b[38;5;"), "16-color fg code: {}", ansi);
    }

    #[test]
    fn test_export_shade_truecolor() {
        let mut canvas = Canvas::new();
        canvas.set(0, 0, Cell {
            ch: blocks::SHADE_DARK,
            fg: Some(Rgb::new(100, 150, 200)),
            bg: None,
        });
        let ansi = to_ansi(&canvas, ColorFormat::TrueColor);
        assert!(ansi.contains("\x1b[38;2;100;150;200m"), "Truecolor fg: {}", ansi);
        assert!(ansi.contains('▓'));
    }

    // --- Fractional fill export tests ---

    #[test]
    fn test_export_fractional_fill_fg_only() {
        let mut canvas = Canvas::new();
        canvas.set(0, 0, Cell {
            ch: blocks::LOWER_1_8,
            fg: RED,
            bg: None,
        });
        let ansi = to_ansi(&canvas, ColorFormat::Color256);
        assert!(ansi.contains('▁'), "Expected ▁: {}", ansi);
        assert!(ansi.contains("\x1b[38;5;1m"), "Expected fg code: {}", ansi);
    }

    #[test]
    fn test_export_fractional_fill_256() {
        let mut canvas = Canvas::new();
        canvas.set(0, 0, Cell {
            ch: blocks::LEFT_3_4,
            fg: Some(Rgb::new(0, 205, 205)),
            bg: None,
        });
        let ansi = to_ansi(&canvas, ColorFormat::Color256);
        assert!(ansi.contains('▊'), "Expected ▊: {}", ansi);
        assert!(ansi.contains("\x1b[38;5;"), "Expected 256 fg code: {}", ansi);
    }

    // --- Full block export test ---

    #[test]
    fn test_export_full_block_fg_bg() {
        // Full block: fg determines visible color, bg should also be present if set
        let mut canvas = Canvas::new();
        canvas.set(0, 0, Cell {
            ch: blocks::FULL,
            fg: RED,
            bg: Some(Rgb::new(0, 0, 238)),
        });
        let ansi = to_ansi(&canvas, ColorFormat::Color256);
        assert!(ansi.contains('█'));
        // Full block passes through non-half-block path: fg and bg both emitted
        assert!(ansi.contains("38;5;"), "Expected fg: {}", ansi);
    }

    // --- Plain text all blocks ---

    #[test]
    fn test_export_all_blocks_plain_text() {
        let mut canvas = Canvas::new();
        for (i, &ch) in blocks::ALL.iter().enumerate() {
            canvas.set(i, 0, Cell {
                ch,
                fg: RED,
                bg: None,
            });
        }
        let text = to_plain_text(&canvas);
        for &ch in &blocks::ALL {
            assert!(text.contains(ch), "Missing block {} in plain text: {}", ch, text);
        }
    }

    // --- Half-block all formats ---

    #[test]
    fn test_export_half_block_all_formats() {
        let red = Rgb { r: 205, g: 0, b: 0 };
        let blue = Rgb::new(0, 0, 238);
        let cell = Cell {
            ch: blocks::UPPER_HALF,
            fg: Some(red),
            bg: Some(blue),
        };

        let mut canvas = Canvas::new();
        canvas.set(0, 0, cell);

        for format in [ColorFormat::TrueColor, ColorFormat::Color256, ColorFormat::Color16] {
            let ansi = to_ansi(&canvas, format);
            assert!(ansi.contains('▀'), "Expected ▀ in {:?}: {}", format, ansi);
            assert!(ansi.contains("\x1b["), "Expected escape codes in {:?}", format);
            assert!(ansi.contains("\x1b[0m"), "Expected reset in {:?}", format);
        }
    }

    // --- Empty canvas export ---

    #[test]
    fn test_export_empty_canvas_ansi() {
        let canvas = Canvas::new();
        let ansi = to_ansi(&canvas, ColorFormat::Color256);
        assert!(ansi.is_empty(), "Expected empty string for empty canvas");
    }

    // --- PNG Export Tests (Sprint 2, Task 2.6) ---

    const CW: u32 = 8;
    const CH: u32 = 16;

    fn red_rgb() -> Rgb { Rgb::new(255, 0, 0) }
    fn blue_rgb() -> Rgb { Rgb::new(0, 0, 255) }

    #[test]
    fn test_png_empty_canvas() {
        let canvas = Canvas::new();
        let img = to_png(&canvas, CW, CH, 1, true);
        // Empty canvas with crop returns 1x1
        assert_eq!(img.width(), 1);
        assert_eq!(img.height(), 1);
    }

    #[test]
    fn test_png_full_block_fills_entire_cell() {
        let mut canvas = Canvas::new();
        canvas.set(0, 0, Cell { ch: blocks::FULL, fg: Some(red_rgb()), bg: None });
        let img = to_png(&canvas, CW, CH, 1, true);
        assert_eq!(img.width(), CW);
        assert_eq!(img.height(), CH);
        // Every pixel should be red (fg)
        for y in 0..CH {
            for x in 0..CW {
                assert_eq!(img.get_pixel(x, y), &Rgba([255, 0, 0, 255]),
                    "Pixel ({},{}) should be red", x, y);
            }
        }
    }

    #[test]
    fn test_png_upper_half() {
        let mut canvas = Canvas::new();
        canvas.set(0, 0, Cell {
            ch: blocks::UPPER_HALF,
            fg: Some(red_rgb()),
            bg: Some(blue_rgb()),
        });
        let img = to_png(&canvas, CW, CH, 1, true);
        let half = CH / 2;
        // Top half: fg (red)
        assert_eq!(img.get_pixel(0, 0), &Rgba([255, 0, 0, 255]));
        assert_eq!(img.get_pixel(0, half - 1), &Rgba([255, 0, 0, 255]));
        // Bottom half: bg (blue)
        assert_eq!(img.get_pixel(0, half), &Rgba([0, 0, 255, 255]));
        assert_eq!(img.get_pixel(0, CH - 1), &Rgba([0, 0, 255, 255]));
    }

    #[test]
    fn test_png_lower_half() {
        let mut canvas = Canvas::new();
        canvas.set(0, 0, Cell {
            ch: blocks::LOWER_HALF,
            fg: Some(red_rgb()),
            bg: Some(blue_rgb()),
        });
        let img = to_png(&canvas, CW, CH, 1, true);
        let half = CH / 2;
        // Top half: bg (blue)
        assert_eq!(img.get_pixel(0, 0), &Rgba([0, 0, 255, 255]));
        // Bottom half: fg (red)
        assert_eq!(img.get_pixel(0, half), &Rgba([255, 0, 0, 255]));
        assert_eq!(img.get_pixel(0, CH - 1), &Rgba([255, 0, 0, 255]));
    }

    #[test]
    fn test_png_left_half() {
        let mut canvas = Canvas::new();
        canvas.set(0, 0, Cell {
            ch: blocks::LEFT_HALF,
            fg: Some(red_rgb()),
            bg: Some(blue_rgb()),
        });
        let img = to_png(&canvas, CW, CH, 1, true);
        let half = CW / 2;
        // Left half: fg (red)
        assert_eq!(img.get_pixel(0, 0), &Rgba([255, 0, 0, 255]));
        assert_eq!(img.get_pixel(half - 1, 0), &Rgba([255, 0, 0, 255]));
        // Right half: bg (blue)
        assert_eq!(img.get_pixel(half, 0), &Rgba([0, 0, 255, 255]));
        assert_eq!(img.get_pixel(CW - 1, 0), &Rgba([0, 0, 255, 255]));
    }

    #[test]
    fn test_png_right_half() {
        let mut canvas = Canvas::new();
        canvas.set(0, 0, Cell {
            ch: blocks::RIGHT_HALF,
            fg: Some(red_rgb()),
            bg: Some(blue_rgb()),
        });
        let img = to_png(&canvas, CW, CH, 1, true);
        let half = CW / 2;
        // Left half: bg (blue)
        assert_eq!(img.get_pixel(0, 0), &Rgba([0, 0, 255, 255]));
        // Right half: fg (red)
        assert_eq!(img.get_pixel(half, 0), &Rgba([255, 0, 0, 255]));
    }

    #[test]
    fn test_png_shade_light_density() {
        let mut canvas = Canvas::new();
        canvas.set(0, 0, Cell {
            ch: blocks::SHADE_LIGHT,
            fg: Some(red_rgb()),
            bg: Some(blue_rgb()),
        });
        let img = to_png(&canvas, CW, CH, 1, true);
        let mut fg_count = 0u32;
        let total = CW * CH;
        for y in 0..CH {
            for x in 0..CW {
                if *img.get_pixel(x, y) == Rgba([255, 0, 0, 255]) {
                    fg_count += 1;
                }
            }
        }
        // Light shade: ~25% fg
        let pct = (fg_count as f32 / total as f32) * 100.0;
        assert!(pct > 15.0 && pct < 35.0, "Light shade fg density {:.1}% should be ~25%", pct);
    }

    #[test]
    fn test_png_shade_medium_density() {
        let mut canvas = Canvas::new();
        canvas.set(0, 0, Cell {
            ch: blocks::SHADE_MEDIUM,
            fg: Some(red_rgb()),
            bg: Some(blue_rgb()),
        });
        let img = to_png(&canvas, CW, CH, 1, true);
        let mut fg_count = 0u32;
        let total = CW * CH;
        for y in 0..CH {
            for x in 0..CW {
                if *img.get_pixel(x, y) == Rgba([255, 0, 0, 255]) {
                    fg_count += 1;
                }
            }
        }
        // Medium shade: ~50% fg
        let pct = (fg_count as f32 / total as f32) * 100.0;
        assert!(pct > 40.0 && pct < 60.0, "Medium shade fg density {:.1}% should be ~50%", pct);
    }

    #[test]
    fn test_png_shade_dark_density() {
        let mut canvas = Canvas::new();
        canvas.set(0, 0, Cell {
            ch: blocks::SHADE_DARK,
            fg: Some(red_rgb()),
            bg: Some(blue_rgb()),
        });
        let img = to_png(&canvas, CW, CH, 1, true);
        let mut fg_count = 0u32;
        let total = CW * CH;
        for y in 0..CH {
            for x in 0..CW {
                if *img.get_pixel(x, y) == Rgba([255, 0, 0, 255]) {
                    fg_count += 1;
                }
            }
        }
        // Dark shade: ~75% fg
        let pct = (fg_count as f32 / total as f32) * 100.0;
        assert!(pct > 65.0 && pct < 85.0, "Dark shade fg density {:.1}% should be ~75%", pct);
    }

    #[test]
    fn test_png_vertical_fractional_lower_1_4() {
        let mut canvas = Canvas::new();
        canvas.set(0, 0, Cell {
            ch: blocks::LOWER_1_4,
            fg: Some(red_rgb()),
            bg: Some(blue_rgb()),
        });
        let img = to_png(&canvas, CW, CH, 1, true);
        // Bottom quarter should be fg (red), top 3/4 should be bg (blue)
        // LOWER_1_4 = 2/8 = 25%, so fg_rows = round(16 * 0.25) = 4
        assert_eq!(img.get_pixel(0, 0), &Rgba([0, 0, 255, 255]), "Top should be bg");
        assert_eq!(img.get_pixel(0, CH - 1), &Rgba([255, 0, 0, 255]), "Bottom should be fg");
    }

    #[test]
    fn test_png_horizontal_fractional_left_3_4() {
        let mut canvas = Canvas::new();
        canvas.set(0, 0, Cell {
            ch: blocks::LEFT_3_4,
            fg: Some(red_rgb()),
            bg: Some(blue_rgb()),
        });
        let img = to_png(&canvas, CW, CH, 1, true);
        // Left 3/4 should be fg (red), right 1/4 should be bg (blue)
        // LEFT_3_4 = 6/8 = 75%, so fg_cols = round(8 * 0.75) = 6
        assert_eq!(img.get_pixel(0, 0), &Rgba([255, 0, 0, 255]), "Left should be fg");
        assert_eq!(img.get_pixel(CW - 1, 0), &Rgba([0, 0, 255, 255]), "Right should be bg");
    }

    #[test]
    fn test_png_autocrop() {
        let mut canvas = Canvas::new_with_size(16, 16);
        // Place a single cell at (5, 3)
        canvas.set(5, 3, Cell { ch: blocks::FULL, fg: Some(red_rgb()), bg: None });
        let img = to_png(&canvas, CW, CH, 1, true);
        // Cropped to 1 cell
        assert_eq!(img.width(), CW);
        assert_eq!(img.height(), CH);
    }

    #[test]
    fn test_png_no_crop() {
        let mut canvas = Canvas::new_with_size(16, 16);
        canvas.set(5, 3, Cell { ch: blocks::FULL, fg: Some(red_rgb()), bg: None });
        let img = to_png(&canvas, CW, CH, 1, false);
        // Full canvas dimensions
        assert_eq!(img.width(), 16 * CW);
        assert_eq!(img.height(), 16 * CH);
    }

    #[test]
    fn test_png_scale_2x() {
        let mut canvas = Canvas::new();
        canvas.set(0, 0, Cell { ch: blocks::FULL, fg: Some(red_rgb()), bg: None });
        let img = to_png(&canvas, CW, CH, 2, true);
        // Doubled dimensions
        assert_eq!(img.width(), CW * 2);
        assert_eq!(img.height(), CH * 2);
        // All pixels should still be red (nearest-neighbor preserves color)
        assert_eq!(img.get_pixel(0, 0), &Rgba([255, 0, 0, 255]));
        assert_eq!(img.get_pixel(CW * 2 - 1, CH * 2 - 1), &Rgba([255, 0, 0, 255]));
    }

    #[test]
    fn test_png_transparent_bg() {
        let mut canvas = Canvas::new();
        // Cell with fg but no bg → bg pixels should be transparent (alpha=0)
        canvas.set(0, 0, Cell {
            ch: blocks::UPPER_HALF,
            fg: Some(red_rgb()),
            bg: None,
        });
        let img = to_png(&canvas, CW, CH, 1, true);
        // Bottom half should be transparent (bg=None)
        let bottom_pixel = img.get_pixel(0, CH - 1);
        assert_eq!(bottom_pixel[3], 0, "bg=None should produce alpha=0, got {:?}", bottom_pixel);
    }

    #[test]
    fn test_png_custom_cell_size() {
        let mut canvas = Canvas::new();
        canvas.set(0, 0, Cell { ch: blocks::FULL, fg: Some(red_rgb()), bg: None });
        let img = to_png(&canvas, 4, 8, 1, true);
        assert_eq!(img.width(), 4);
        assert_eq!(img.height(), 8);
    }

    #[test]
    fn test_png_space_fills_bg() {
        let mut canvas = Canvas::new_with_size(2, 2);
        canvas.set(0, 0, Cell { ch: ' ', fg: None, bg: Some(blue_rgb()) });
        // Cell::is_empty only checks ch==' ', so space+bg is "empty" for bounding box.
        // Use crop=false to test space rendering directly.
        let img = to_png(&canvas, CW, CH, 1, false);
        // Space should fill with bg color
        assert_eq!(img.get_pixel(0, 0), &Rgba([0, 0, 255, 255]));
    }
}
