use std::collections::HashMap;
use std::path::Path;

use image::GenericImageView;

use crate::cell::{self, blocks, Cell, Rgb};

/// How the image should be scaled to fit the canvas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FitMode {
    FitToCanvas,
    #[allow(dead_code)] // Used in Task 2.4 (ImportOptions dialog)
    CustomSize(usize, usize),
}

/// Color quantization mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportColorMode {
    Color256,
    Color16,
}

/// Character set for rasterization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportCharSet {
    FullBlocks,
    HalfBlocks,
}

/// Import configuration.
pub struct ImportOptions {
    pub fit_mode: FitMode,
    pub color_mode: ImportColorMode,
    pub char_set: ImportCharSet,
}

impl Default for ImportOptions {
    fn default() -> Self {
        ImportOptions {
            fit_mode: FitMode::FitToCanvas,
            color_mode: ImportColorMode::Color256,
            char_set: ImportCharSet::HalfBlocks,
        }
    }
}

/// Errors during image import.
#[derive(Debug)]
pub enum ImportError {
    FileNotFound,
    InvalidFormat(String),
    DecodeFailed(String),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportError::FileNotFound => write!(f, "File not found"),
            ImportError::InvalidFormat(msg) => write!(f, "Invalid format: {}", msg),
            ImportError::DecodeFailed(msg) => write!(f, "Decode failed: {}", msg),
        }
    }
}

/// Find the nearest xterm-256 index among the first 16 colors only.
fn nearest_16(color: &Rgb) -> u8 {
    let mut best_idx: u8 = 0;
    let mut best_dist = u32::MAX;
    for i in 0u8..16 {
        let c = cell::color256_to_rgb(i);
        let dr = color.r as i32 - c.r as i32;
        let dg = color.g as i32 - c.g as i32;
        let db = color.b as i32 - c.b as i32;
        let dist = (dr * dr + dg * dg + db * db) as u32;
        if dist < best_dist {
            best_dist = dist;
            best_idx = i;
        }
    }
    best_idx
}

/// Quantize an RGB pixel to an xterm-256 Rgb value, using a cache.
fn quantize(
    r: u8,
    g: u8,
    b: u8,
    color_mode: ImportColorMode,
    cache: &mut HashMap<(u8, u8, u8), Rgb>,
) -> Rgb {
    if let Some(&cached) = cache.get(&(r, g, b)) {
        return cached;
    }
    let src = Rgb::new(r, g, b);
    let idx = match color_mode {
        ImportColorMode::Color256 => cell::nearest_256(&src),
        ImportColorMode::Color16 => nearest_16(&src),
    };
    let result = cell::color256_to_rgb(idx);
    cache.insert((r, g, b), result);
    result
}

/// Import an image and convert it to a cell grid.
///
/// `target_width` and `target_height` are in cell space.
/// For HalfBlocks mode, the internal pixel height is `target_height * 2`
/// but the returned grid is always `target_width × target_height` cells.
pub fn import_image(
    path: &Path,
    target_width: usize,
    target_height: usize,
    options: &ImportOptions,
) -> Result<Vec<Vec<Cell>>, ImportError> {
    if !path.exists() {
        return Err(ImportError::FileNotFound);
    }

    // Decode image
    let img = image::open(path).map_err(|e| ImportError::DecodeFailed(e.to_string()))?;

    let (src_w, src_h) = img.dimensions();
    if src_w == 0 || src_h == 0 {
        return Err(ImportError::InvalidFormat("Image has zero dimensions".to_string()));
    }

    // Determine target pixel dimensions
    let (cell_w, cell_h) = match options.fit_mode {
        FitMode::FitToCanvas => (target_width, target_height),
        FitMode::CustomSize(w, h) => (w, h),
    };

    if cell_w == 0 || cell_h == 0 {
        return Err(ImportError::InvalidFormat(
            "Target dimensions must be greater than zero".to_string(),
        ));
    }

    // Pixel-space target for downscale
    let px_w = cell_w;
    let px_h = match options.char_set {
        ImportCharSet::FullBlocks => cell_h,
        ImportCharSet::HalfBlocks => cell_h
            .checked_mul(2)
            .ok_or_else(|| ImportError::InvalidFormat("Target height overflow".to_string()))?,
    };

    // Compute aspect-ratio-preserving dimensions and letterbox offsets
    let (scaled_w, scaled_h, offset_x, offset_y) =
        compute_fit(src_w as usize, src_h as usize, px_w, px_h);

    // Downscale image to the scaled dimensions
    let resized = image::imageops::resize(
        &img,
        scaled_w as u32,
        scaled_h as u32,
        image::imageops::FilterType::Lanczos3,
    );

    // Build pixel grid (px_w × px_h) with letterbox
    let mut pixels: Vec<Vec<Option<(u8, u8, u8)>>> = vec![vec![None; px_w]; px_h];

    for py in 0..scaled_h {
        for px in 0..scaled_w {
            let dest_x = px + offset_x;
            let dest_y = py + offset_y;
            if dest_x < px_w && dest_y < px_h {
                let pixel = resized.get_pixel(px as u32, py as u32);
                let [r, g, b, a] = pixel.0;
                if a >= 128 {
                    pixels[dest_y][dest_x] = Some((r, g, b));
                }
                // else: transparent → stays None
            }
        }
    }

    // Rasterize to cells
    let mut cache: HashMap<(u8, u8, u8), Rgb> = HashMap::new();

    let cells = match options.char_set {
        ImportCharSet::FullBlocks => {
            rasterize_full_blocks(&pixels, cell_w, cell_h, options.color_mode, &mut cache)
        }
        ImportCharSet::HalfBlocks => {
            rasterize_half_blocks(&pixels, cell_w, cell_h, options.color_mode, &mut cache)
        }
    };

    Ok(cells)
}

/// Compute the scaled image dimensions that fit within the target while preserving aspect ratio.
/// Returns (scaled_w, scaled_h, offset_x, offset_y) for letterboxing.
fn compute_fit(
    src_w: usize,
    src_h: usize,
    target_w: usize,
    target_h: usize,
) -> (usize, usize, usize, usize) {
    if src_w == 0 || src_h == 0 || target_w == 0 || target_h == 0 {
        return (0, 0, 0, 0);
    }

    let scale_x = target_w as f64 / src_w as f64;
    let scale_y = target_h as f64 / src_h as f64;
    let scale = scale_x.min(scale_y);

    let scaled_w = (src_w as f64 * scale).round() as usize;
    let scaled_h = (src_h as f64 * scale).round() as usize;

    // Ensure at least 1 pixel and don't exceed target
    let scaled_w = scaled_w.clamp(1, target_w);
    let scaled_h = scaled_h.clamp(1, target_h);

    let offset_x = (target_w - scaled_w) / 2;
    let offset_y = (target_h - scaled_h) / 2;

    (scaled_w, scaled_h, offset_x, offset_y)
}

/// Rasterize to full-block cells: each pixel → one cell with bg color.
fn rasterize_full_blocks(
    pixels: &[Vec<Option<(u8, u8, u8)>>],
    cell_w: usize,
    cell_h: usize,
    color_mode: ImportColorMode,
    cache: &mut HashMap<(u8, u8, u8), Rgb>,
) -> Vec<Vec<Cell>> {
    let mut cells = vec![vec![Cell::empty(); cell_w]; cell_h];
    for y in 0..cell_h {
        for x in 0..cell_w {
            if y < pixels.len() && x < pixels[y].len() {
                if let Some((r, g, b)) = pixels[y][x] {
                    let rgb = quantize(r, g, b, color_mode, cache);
                    cells[y][x] = Cell {
                        ch: ' ',
                        fg: None,
                        bg: Some(rgb),
                    };
                }
            }
        }
    }
    cells
}

/// Rasterize to half-block cells: two pixel rows → one cell row using ▀/▄.
fn rasterize_half_blocks(
    pixels: &[Vec<Option<(u8, u8, u8)>>],
    cell_w: usize,
    cell_h: usize,
    color_mode: ImportColorMode,
    cache: &mut HashMap<(u8, u8, u8), Rgb>,
) -> Vec<Vec<Cell>> {
    let mut cells = vec![vec![Cell::empty(); cell_w]; cell_h];
    for cy in 0..cell_h {
        let upper_row = cy * 2;
        let lower_row = cy * 2 + 1;
        for cx in 0..cell_w {
            let upper = pixels
                .get(upper_row)
                .and_then(|row| row.get(cx))
                .copied()
                .flatten();
            let lower = pixels
                .get(lower_row)
                .and_then(|row| row.get(cx))
                .copied()
                .flatten();

            cells[cy][cx] = match (upper, lower) {
                (None, None) => Cell::empty(),
                (Some((r, g, b)), None) => {
                    let rgb = quantize(r, g, b, color_mode, cache);
                    Cell {
                        ch: blocks::UPPER_HALF,
                        fg: Some(rgb),
                        bg: None,
                    }
                }
                (None, Some((r, g, b))) => {
                    let rgb = quantize(r, g, b, color_mode, cache);
                    Cell {
                        ch: blocks::LOWER_HALF,
                        fg: Some(rgb),
                        bg: None,
                    }
                }
                (Some((ur, ug, ub)), Some((lr, lg, lb))) => {
                    let upper_rgb = quantize(ur, ug, ub, color_mode, cache);
                    let lower_rgb = quantize(lr, lg, lb, color_mode, cache);
                    Cell {
                        ch: blocks::UPPER_HALF,
                        fg: Some(upper_rgb),
                        bg: Some(lower_rgb),
                    }
                }
            };
        }
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Helper: create a temp PNG file from raw RGBA pixels.
    fn write_test_png(
        path: &Path,
        width: u32,
        height: u32,
        pixels: &[(u8, u8, u8, u8)],
    ) {
        let mut img = image::RgbaImage::new(width, height);
        for (i, &(r, g, b, a)) in pixels.iter().enumerate() {
            let x = (i as u32) % width;
            let y = (i as u32) / width;
            if x < width && y < height {
                img.put_pixel(x, y, image::Rgba([r, g, b, a]));
            }
        }
        img.save(path).unwrap();
    }

    /// Helper: create a temp GIF file with multiple frames.
    fn write_test_gif(path: &Path, width: u16, height: u16, frame_colors: &[(u8, u8, u8)]) {
        use std::fs::File;
        let file = File::create(path).unwrap();
        let mut encoder = image::codecs::gif::GifEncoder::new(file);
        for &(r, g, b) in frame_colors {
            let pixels: Vec<u8> = (0..(width as u32 * height as u32))
                .flat_map(|_| vec![r, g, b, 255])
                .collect();
            let frame = image::Frame::new(
                image::RgbaImage::from_raw(width as u32, height as u32, pixels).unwrap(),
            );
            encoder.encode_frames(std::iter::once(frame)).unwrap();
        }
    }

    #[test]
    fn test_full_block_rasterize() {
        let dir = std::env::temp_dir().join("kakukuma_test_import");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("solid_4x4.png");

        // Create a solid red 4×4 image
        let red_pixel = (255, 0, 0, 255);
        let pixels: Vec<_> = vec![red_pixel; 16];
        write_test_png(&path, 4, 4, &pixels);

        let opts = ImportOptions {
            fit_mode: FitMode::FitToCanvas,
            color_mode: ImportColorMode::Color256,
            char_set: ImportCharSet::FullBlocks,
        };
        let cells = import_image(&path, 4, 4, &opts).unwrap();
        assert_eq!(cells.len(), 4);
        assert_eq!(cells[0].len(), 4);

        // Every cell should have bg color (quantized red), no fg, space char
        for row in &cells {
            for cell in row {
                assert_eq!(cell.ch, ' ');
                assert!(cell.fg.is_none());
                assert!(cell.bg.is_some());
                // Quantized red should be close to xterm red
                let bg = cell.bg.unwrap();
                assert!(bg.r > 100, "Red channel should be high, got {}", bg.r);
            }
        }
    }

    #[test]
    fn test_half_block_rasterize() {
        let dir = std::env::temp_dir().join("kakukuma_test_import");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("half_4x4.png");

        // 4×4 image → at half-block: 4×2 cells (4 pixel cols, 4 pixel rows → 2 cell rows)
        let red = (255, 0, 0, 255);
        let blue = (0, 0, 255, 255);
        // Row 0-1: red (upper) + blue (lower) for cell row 0
        // Row 2-3: blue (upper) + red (lower) for cell row 1
        let pixels = vec![
            red, red, red, red,
            blue, blue, blue, blue,
            blue, blue, blue, blue,
            red, red, red, red,
        ];
        write_test_png(&path, 4, 4, &pixels);

        let opts = ImportOptions {
            fit_mode: FitMode::FitToCanvas,
            color_mode: ImportColorMode::Color256,
            char_set: ImportCharSet::HalfBlocks,
        };
        let cells = import_image(&path, 4, 2, &opts).unwrap();
        assert_eq!(cells.len(), 2);
        assert_eq!(cells[0].len(), 4);

        // Cell row 0: ▀ with fg=red, bg=blue
        for cell in &cells[0] {
            assert_eq!(cell.ch, blocks::UPPER_HALF);
            assert!(cell.fg.is_some());
            assert!(cell.bg.is_some());
        }
    }

    #[test]
    fn test_half_block_mixed_alpha() {
        let dir = std::env::temp_dir().join("kakukuma_test_import");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("mixed_alpha.png");

        // 1×2 image: upper pixel transparent, lower pixel blue
        let transparent = (0, 0, 0, 0);
        let blue = (0, 0, 255, 255);
        write_test_png(&path, 1, 2, &[transparent, blue]);

        let opts = ImportOptions {
            fit_mode: FitMode::FitToCanvas,
            color_mode: ImportColorMode::Color256,
            char_set: ImportCharSet::HalfBlocks,
        };
        let cells = import_image(&path, 1, 1, &opts).unwrap();
        assert_eq!(cells.len(), 1);
        assert_eq!(cells[0].len(), 1);

        // Upper transparent + lower visible → ▄ with fg=lower color, bg=None
        let cell = &cells[0][0];
        assert_eq!(cell.ch, blocks::LOWER_HALF);
        assert!(cell.fg.is_some());
        assert!(cell.bg.is_none());
    }

    #[test]
    fn test_aspect_ratio_letterbox() {
        let dir = std::env::temp_dir().join("kakukuma_test_import");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("wide_8x2.png");

        // Wide image: 8×2 → fit into 4×4 full-block canvas
        // Should letterbox vertically
        let red = (255, 0, 0, 255);
        let pixels: Vec<_> = vec![red; 16];
        write_test_png(&path, 8, 2, &pixels);

        let opts = ImportOptions {
            fit_mode: FitMode::FitToCanvas,
            color_mode: ImportColorMode::Color256,
            char_set: ImportCharSet::FullBlocks,
        };
        let cells = import_image(&path, 4, 4, &opts).unwrap();
        assert_eq!(cells.len(), 4);
        assert_eq!(cells[0].len(), 4);

        // Top and bottom rows should be letterbox (empty cells)
        // The image should be centered
        let top_empty = cells[0].iter().all(|c| c.bg.is_none());
        let bottom_empty = cells[3].iter().all(|c| c.bg.is_none());
        assert!(top_empty, "Top row should be letterbox (empty)");
        assert!(bottom_empty, "Bottom row should be letterbox (empty)");

        // Middle rows should have content
        let mid_has_content = cells[1].iter().any(|c| c.bg.is_some())
            || cells[2].iter().any(|c| c.bg.is_some());
        assert!(mid_has_content, "Middle rows should have content");
    }

    #[test]
    fn test_transparency() {
        let dir = std::env::temp_dir().join("kakukuma_test_import");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("alpha_2x2.png");

        // 2×2 with top-left opaque, rest transparent
        let red = (255, 0, 0, 255);
        let transparent = (0, 0, 0, 64); // alpha < 128
        write_test_png(&path, 2, 2, &[red, transparent, transparent, transparent]);

        let opts = ImportOptions {
            fit_mode: FitMode::FitToCanvas,
            color_mode: ImportColorMode::Color256,
            char_set: ImportCharSet::FullBlocks,
        };
        let cells = import_image(&path, 2, 2, &opts).unwrap();

        // Top-left should have color
        assert!(cells[0][0].bg.is_some());
        // Others should be empty (transparent)
        assert!(cells[0][1].bg.is_none());
        assert!(cells[1][0].bg.is_none());
        assert!(cells[1][1].bg.is_none());
    }

    #[test]
    fn test_gif_first_frame() {
        let dir = std::env::temp_dir().join("kakukuma_test_import");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("animated.gif");

        // 2-frame GIF: first frame red, second frame blue
        write_test_gif(&path, 2, 2, &[(255, 0, 0), (0, 0, 255)]);

        let opts = ImportOptions {
            fit_mode: FitMode::FitToCanvas,
            color_mode: ImportColorMode::Color256,
            char_set: ImportCharSet::FullBlocks,
        };
        let cells = import_image(&path, 2, 2, &opts).unwrap();

        // Should have decoded (first frame = red)
        assert_eq!(cells.len(), 2);
        assert_eq!(cells[0].len(), 2);
        // Should have content from first frame
        let cell = &cells[0][0];
        assert!(cell.bg.is_some());
        let bg = cell.bg.unwrap();
        // First frame is red
        assert!(bg.r > 100, "Expected red first frame, got r={}", bg.r);
    }

    #[test]
    fn test_invalid_file() {
        let path = Path::new("/nonexistent/path/image.png");
        let opts = ImportOptions::default();
        let result = import_image(path, 4, 4, &opts);
        assert!(matches!(result, Err(ImportError::FileNotFound)));
    }

    #[test]
    fn test_corrupt_file() {
        let dir = std::env::temp_dir().join("kakukuma_test_import");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("corrupt.png");

        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(b"not a real image file").unwrap();

        let opts = ImportOptions::default();
        let result = import_image(&path, 4, 4, &opts);
        assert!(matches!(result, Err(ImportError::DecodeFailed(_))));
    }

    #[test]
    fn test_color_quantization() {
        // Pure red (255,0,0) should quantize to xterm index 196 or 9
        let rgb = Rgb::new(255, 0, 0);
        let idx = cell::nearest_256(&rgb);
        let quantized = cell::color256_to_rgb(idx);
        // The quantized value should be close to red
        assert!(quantized.r > 200);
        assert!(quantized.g < 50);
        assert!(quantized.b < 50);
    }

    #[test]
    fn test_16_color_mode() {
        let dir = std::env::temp_dir().join("kakukuma_test_import");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("color16.png");

        let red = (255, 0, 0, 255);
        write_test_png(&path, 2, 2, &[red, red, red, red]);

        let opts = ImportOptions {
            fit_mode: FitMode::FitToCanvas,
            color_mode: ImportColorMode::Color16,
            char_set: ImportCharSet::FullBlocks,
        };
        let cells = import_image(&path, 2, 2, &opts).unwrap();

        let bg = cells[0][0].bg.unwrap();
        // 16-color quantized red should match one of the 16 ANSI colors
        let idx = nearest_16(&Rgb::new(255, 0, 0));
        assert!(idx < 16, "Should quantize to ANSI 16 range");
        let expected = cell::color256_to_rgb(idx);
        assert_eq!(bg, expected);
    }

    #[test]
    fn test_downscale_before_quantize() {
        let dir = std::env::temp_dir().join("kakukuma_test_import");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("large_32x32.png");

        // Large 32×32 image → target 4×4 cells
        let green = (0, 255, 0, 255);
        let pixels: Vec<_> = vec![green; 32 * 32];
        write_test_png(&path, 32, 32, &pixels);

        let opts = ImportOptions {
            fit_mode: FitMode::FitToCanvas,
            color_mode: ImportColorMode::Color256,
            char_set: ImportCharSet::FullBlocks,
        };
        let cells = import_image(&path, 4, 4, &opts).unwrap();

        // Output grid should match target, not source
        assert_eq!(cells.len(), 4);
        assert_eq!(cells[0].len(), 4);
    }

    #[test]
    fn test_compute_fit_square() {
        let (sw, sh, ox, oy) = compute_fit(100, 100, 10, 10);
        assert_eq!((sw, sh), (10, 10));
        assert_eq!((ox, oy), (0, 0));
    }

    #[test]
    fn test_compute_fit_wide() {
        let (sw, sh, ox, oy) = compute_fit(200, 100, 10, 10);
        assert_eq!(sw, 10);
        assert_eq!(sh, 5);
        assert_eq!(ox, 0);
        assert!(oy > 0, "Should have vertical offset for letterbox");
    }

    #[test]
    fn test_compute_fit_tall() {
        let (sw, sh, ox, oy) = compute_fit(100, 200, 10, 10);
        assert_eq!(sw, 5);
        assert_eq!(sh, 10);
        assert!(ox > 0, "Should have horizontal offset for letterbox");
        assert_eq!(oy, 0);
    }
}
