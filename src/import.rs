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
    TrueColor,
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
    /// Boost color saturation to survive 256-palette quantization.
    /// 1.0 = no change, 2.0 = double saturation, etc.
    pub color_boost: f32,
    /// Prefer chromatic palette matches over grays when source has hue.
    pub preserve_hue: bool,
    /// Auto-normalize brightness (stretch histogram to fill 0-255 range).
    pub normalize: bool,
    /// Posterize: reduce to N distinct colors via k-means clustering.
    /// None = off (keep all colors). Some(N) = reduce to N colors (2-64).
    pub posterize: Option<usize>,
}

impl Default for ImportOptions {
    fn default() -> Self {
        ImportOptions {
            fit_mode: FitMode::FitToCanvas,
            color_mode: ImportColorMode::TrueColor,
            char_set: ImportCharSet::HalfBlocks,
            color_boost: 1.0,
            preserve_hue: true,
            normalize: true,
            posterize: None,
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

/// Auto-levels: stretch the brightness range of the pixel grid to fill 0-255.
/// Uses 2nd/98th percentile to avoid outlier sensitivity.
fn normalize_pixels(pixels: &mut [Vec<Option<(u8, u8, u8)>>]) {
    // Collect all channel values
    let mut vals: Vec<u8> = Vec::new();
    for row in pixels.iter() {
        for px in row.iter() {
            if let Some((r, g, b)) = px {
                vals.push(*r);
                vals.push(*g);
                vals.push(*b);
            }
        }
    }
    if vals.is_empty() {
        return;
    }
    vals.sort_unstable();

    // 2nd and 98th percentile
    let lo = vals[vals.len() * 2 / 100] as f32;
    let hi = vals[vals.len() * 98 / 100] as f32;
    let range = hi - lo;
    if range < 10.0 {
        return; // already well-distributed or nearly uniform
    }

    for row in pixels.iter_mut() {
        for px in row.iter_mut() {
            if let Some((r, g, b)) = px {
                let stretch = |v: u8| -> u8 {
                    (((v as f32 - lo) / range) * 255.0).clamp(0.0, 255.0) as u8
                };
                *r = stretch(*r);
                *g = stretch(*g);
                *b = stretch(*b);
            }
        }
    }
}

/// Posterize pixels via k-means clustering: reduce to N distinct colors.
/// Replaces each pixel with the nearest cluster centroid.
fn posterize_pixels(pixels: &mut [Vec<Option<(u8, u8, u8)>>], n_colors: usize) {
    // Collect all opaque pixels
    let mut samples: Vec<(u8, u8, u8)> = Vec::new();
    for row in pixels.iter() {
        for px in row.iter().flatten() {
            samples.push(*px);
        }
    }
    if samples.is_empty() || n_colors == 0 {
        return;
    }
    let n_colors = n_colors.clamp(2, 64);

    // Initialize centroids by sampling evenly from the pixel list
    let step = (samples.len() as f64 / n_colors as f64).max(1.0);
    let mut centroids: Vec<(f64, f64, f64)> = (0..n_colors)
        .map(|i| {
            let idx = ((i as f64 * step) as usize).min(samples.len() - 1);
            let (r, g, b) = samples[idx];
            (r as f64, g as f64, b as f64)
        })
        .collect();

    // K-means iterations (converges fast on small pixel sets)
    let mut assignments: Vec<usize> = vec![0; samples.len()];
    for _ in 0..20 {
        // Assign each pixel to nearest centroid
        let mut changed = false;
        for (i, &(r, g, b)) in samples.iter().enumerate() {
            let (pr, pg, pb) = (r as f64, g as f64, b as f64);
            let mut best = 0;
            let mut best_dist = f64::MAX;
            for (j, &(cr, cg, cb)) in centroids.iter().enumerate() {
                let d = (pr - cr).powi(2) + (pg - cg).powi(2) + (pb - cb).powi(2);
                if d < best_dist {
                    best_dist = d;
                    best = j;
                }
            }
            if assignments[i] != best {
                assignments[i] = best;
                changed = true;
            }
        }
        if !changed {
            break;
        }

        // Recompute centroids
        let mut sums = vec![(0.0f64, 0.0f64, 0.0f64); n_colors];
        let mut counts = vec![0usize; n_colors];
        for (i, &(r, g, b)) in samples.iter().enumerate() {
            let c = assignments[i];
            sums[c].0 += r as f64;
            sums[c].1 += g as f64;
            sums[c].2 += b as f64;
            counts[c] += 1;
        }
        for j in 0..n_colors {
            if counts[j] > 0 {
                centroids[j] = (
                    sums[j].0 / counts[j] as f64,
                    sums[j].1 / counts[j] as f64,
                    sums[j].2 / counts[j] as f64,
                );
            }
        }
    }

    // Build lookup: original color → nearest centroid (as u8)
    let final_centroids: Vec<(u8, u8, u8)> = centroids
        .iter()
        .map(|&(r, g, b)| (r.round() as u8, g.round() as u8, b.round() as u8))
        .collect();

    // Replace pixels with their centroid color
    for row in pixels.iter_mut() {
        for px in row.iter_mut() {
            if let Some((r, g, b)) = px {
                let (pr, pg, pb) = (*r as f64, *g as f64, *b as f64);
                let mut best = 0;
                let mut best_dist = f64::MAX;
                for (j, &(cr, cg, cb)) in centroids.iter().enumerate() {
                    let d = (pr - cr).powi(2) + (pg - cg).powi(2) + (pb - cb).powi(2);
                    if d < best_dist {
                        best_dist = d;
                        best = j;
                    }
                }
                *px = Some(final_centroids[best]);
            }
        }
    }
}

/// Boost saturation and brightness of an RGB pixel to survive 256-palette quantization.
/// Pushes each channel away from the mean (gray axis) for saturation,
/// and lifts dark chromatic pixels to escape the xterm-256 dead zone (0→95).
fn boost_saturation(r: u8, g: u8, b: u8, factor: f32) -> (u8, u8, u8) {
    if factor <= 1.0 {
        return (r, g, b);
    }
    let (mut fr, mut fg, mut fb) = (r as f32, g as f32, b as f32);

    // Saturation boost: push channels away from the mean
    let mean = (fr + fg + fb) / 3.0;
    fr = mean + (fr - mean) * factor;
    fg = mean + (fg - mean) * factor;
    fb = mean + (fb - mean) * factor;

    // Brightness lift for dark chromatic pixels (skin tones, browns, etc.).
    // The xterm-256 cube has a dead zone from 0-95 per channel — dark colors
    // with hue lose their chromaticity when channels snap to 0.
    // Lift dark pixels proportionally so their channels clear the dead zone.
    let max_ch = fr.max(fg).max(fb);
    let min_ch = fr.min(fg).min(fb);
    let sat = if max_ch > 0.0 { (max_ch - min_ch) / max_ch } else { 0.0 };
    if sat > 0.15 && max_ch < 150.0 && max_ch > 10.0 {
        // Scale to lift the max channel toward 150, preserving ratios
        let lift = (150.0 / max_ch).min(1.5); // cap at 1.5× to avoid blowout
        fr *= lift;
        fg *= lift;
        fb *= lift;
    }

    (
        fr.clamp(0.0, 255.0) as u8,
        fg.clamp(0.0, 255.0) as u8,
        fb.clamp(0.0, 255.0) as u8,
    )
}

/// Quantize an RGB pixel to an xterm-256 Rgb value, using a cache.
fn quantize(
    r: u8,
    g: u8,
    b: u8,
    color_mode: ImportColorMode,
    color_boost: f32,
    preserve_hue: bool,
    cache: &mut HashMap<(u8, u8, u8), Rgb>,
) -> Rgb {
    if let Some(&cached) = cache.get(&(r, g, b)) {
        return cached;
    }
    let (r, g, b) = boost_saturation(r, g, b, color_boost);
    let src = Rgb::new(r, g, b);
    if matches!(color_mode, ImportColorMode::TrueColor) {
        return src;
    }
    let idx = match color_mode {
        ImportColorMode::Color256 if preserve_hue => cell::nearest_256_hue(&src),
        ImportColorMode::Color256 => cell::nearest_256(&src),
        ImportColorMode::Color16 => cell::nearest_16(&src),
        ImportColorMode::TrueColor => unreachable!(),
    };
    let result = cell::color256_to_rgb(idx);
    cache.insert((r, g, b), result);
    result
}

/// Mosaic import: divide source image into a grid, average each region's color.
/// Produces clean, readable pixel art — one solid color per cell.
/// For HalfBlocks, averages top and bottom halves of each cell region separately.
pub fn import_mosaic(
    path: &Path,
    target_width: usize,
    target_height: usize,
    options: &ImportOptions,
) -> Result<Vec<Vec<Cell>>, ImportError> {
    if !path.exists() {
        return Err(ImportError::FileNotFound);
    }

    let img = image::open(path).map_err(|e| ImportError::DecodeFailed(e.to_string()))?;
    let rgba = img.to_rgba8();
    let (src_w, src_h) = (rgba.width() as usize, rgba.height() as usize);
    if src_w == 0 || src_h == 0 {
        return Err(ImportError::InvalidFormat("Image has zero dimensions".to_string()));
    }

    let (cell_w, cell_h) = match options.fit_mode {
        FitMode::FitToCanvas => (target_width, target_height),
        FitMode::CustomSize(w, h) => (w, h),
    };
    if cell_w == 0 || cell_h == 0 {
        return Err(ImportError::InvalidFormat("Target dimensions must be > 0".to_string()));
    }

    // Pixel rows per cell: 2 for half-blocks, 1 for full blocks
    let rows_per_cell = match options.char_set {
        ImportCharSet::HalfBlocks => 2usize,
        ImportCharSet::FullBlocks => 1,
    };
    let grid_rows = cell_h * rows_per_cell;

    // Average a rectangular region of the source image
    let avg_region = |x0: usize, y0: usize, x1: usize, y1: usize| -> Option<(u8, u8, u8)> {
        let mut r_sum: u64 = 0;
        let mut g_sum: u64 = 0;
        let mut b_sum: u64 = 0;
        let mut count: u64 = 0;
        for sy in y0..y1.min(src_h) {
            for sx in x0..x1.min(src_w) {
                let px = rgba.get_pixel(sx as u32, sy as u32);
                let [r, g, b, a] = px.0;
                if a >= 128 {
                    r_sum += r as u64;
                    g_sum += g as u64;
                    b_sum += b as u64;
                    count += 1;
                }
            }
        }
        if count == 0 {
            return None;
        }
        Some((
            (r_sum / count) as u8,
            (g_sum / count) as u8,
            (b_sum / count) as u8,
        ))
    };

    let mut cache: HashMap<(u8, u8, u8), Rgb> = HashMap::new();

    let mut cells = vec![vec![Cell::empty(); cell_w]; cell_h];
    for cy in 0..cell_h {
        for cx in 0..cell_w {
            // Map cell to source region
            let sx0 = cx * src_w / cell_w;
            let sx1 = (cx + 1) * src_w / cell_w;

            match options.char_set {
                ImportCharSet::FullBlocks => {
                    let sy0 = cy * src_h / cell_h;
                    let sy1 = (cy + 1) * src_h / cell_h;
                    if let Some((r, g, b)) = avg_region(sx0, sy0, sx1, sy1) {
                        let rgb = quantize(r, g, b, options.color_mode, options.color_boost, options.preserve_hue, &mut cache);
                        cells[cy][cx] = Cell { ch: '\u{2588}', fg: Some(rgb), bg: None };
                    }
                }
                ImportCharSet::HalfBlocks => {
                    // Top half of cell region
                    let sy_top0 = (cy * 2) * src_h / grid_rows;
                    let sy_top1 = (cy * 2 + 1) * src_h / grid_rows;
                    // Bottom half of cell region
                    let sy_bot0 = (cy * 2 + 1) * src_h / grid_rows;
                    let sy_bot1 = (cy * 2 + 2) * src_h / grid_rows;

                    let top = avg_region(sx0, sy_top0, sx1, sy_top1);
                    let bot = avg_region(sx0, sy_bot0, sx1, sy_bot1);

                    cells[cy][cx] = match (top, bot) {
                        (None, None) => Cell::empty(),
                        (Some((r, g, b)), None) => {
                            let rgb = quantize(r, g, b, options.color_mode, options.color_boost, options.preserve_hue, &mut cache);
                            Cell { ch: blocks::UPPER_HALF, fg: Some(rgb), bg: None }
                        }
                        (None, Some((r, g, b))) => {
                            let rgb = quantize(r, g, b, options.color_mode, options.color_boost, options.preserve_hue, &mut cache);
                            Cell { ch: blocks::LOWER_HALF, fg: Some(rgb), bg: None }
                        }
                        (Some((tr, tg, tb)), Some((br, bg_, bb))) => {
                            let top_rgb = quantize(tr, tg, tb, options.color_mode, options.color_boost, options.preserve_hue, &mut cache);
                            let bot_rgb = quantize(br, bg_, bb, options.color_mode, options.color_boost, options.preserve_hue, &mut cache);
                            Cell { ch: blocks::UPPER_HALF, fg: Some(top_rgb), bg: Some(bot_rgb) }
                        }
                    };
                }
            }
        }
    }

    Ok(cells)
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
    import_dynamic_image(img, target_width, target_height, options)
}

/// Import from raw RGBA pixel data (e.g., clipboard image).
///
/// `rgba_data` must be `width * height * 4` bytes (RGBA order).
pub fn import_image_data(
    rgba_data: &[u8],
    width: u32,
    height: u32,
    target_width: usize,
    target_height: usize,
    options: &ImportOptions,
) -> Result<Vec<Vec<Cell>>, ImportError> {
    let expected = (width as usize) * (height as usize) * 4;
    if rgba_data.len() != expected {
        return Err(ImportError::InvalidFormat(format!(
            "Expected {} bytes for {}x{} RGBA, got {}",
            expected, width, height, rgba_data.len()
        )));
    }
    let img_buf: image::RgbaImage =
        image::ImageBuffer::from_raw(width, height, rgba_data.to_vec())
            .ok_or_else(|| ImportError::DecodeFailed("Failed to create image from RGBA data".to_string()))?;
    let img = image::DynamicImage::ImageRgba8(img_buf);
    import_dynamic_image(img, target_width, target_height, options)
}

/// Shared import pipeline: resize, letterbox, normalize, posterize, rasterize.
fn import_dynamic_image(
    img: image::DynamicImage,
    target_width: usize,
    target_height: usize,
    options: &ImportOptions,
) -> Result<Vec<Vec<Cell>>, ImportError> {
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

    // Auto-levels: stretch brightness to fill 0-255 range
    if options.normalize {
        normalize_pixels(&mut pixels);
    }

    // Posterize: reduce to N distinct colors via k-means
    if let Some(n) = options.posterize {
        posterize_pixels(&mut pixels, n);
    }

    // Rasterize to cells
    let mut cache: HashMap<(u8, u8, u8), Rgb> = HashMap::new();

    let cells = match options.char_set {
        ImportCharSet::FullBlocks => {
            rasterize_full_blocks(&pixels, cell_w, cell_h, options.color_mode, options.color_boost, options.preserve_hue, &mut cache)
        }
        ImportCharSet::HalfBlocks => {
            rasterize_half_blocks(&pixels, cell_w, cell_h, options.color_mode, options.color_boost, options.preserve_hue, &mut cache)
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
    color_boost: f32,
    preserve_hue: bool,
    cache: &mut HashMap<(u8, u8, u8), Rgb>,
) -> Vec<Vec<Cell>> {
    let mut cells = vec![vec![Cell::empty(); cell_w]; cell_h];
    for y in 0..cell_h {
        for x in 0..cell_w {
            if y < pixels.len() && x < pixels[y].len() {
                if let Some((r, g, b)) = pixels[y][x] {
                    let rgb = quantize(r, g, b, color_mode, color_boost, preserve_hue, cache);
                    cells[y][x] = Cell {
                        ch: '\u{2588}', // █ full block
                        fg: Some(rgb),
                        bg: None,
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
    color_boost: f32,
    preserve_hue: bool,
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
                    let rgb = quantize(r, g, b, color_mode, color_boost, preserve_hue, cache);
                    Cell {
                        ch: blocks::UPPER_HALF,
                        fg: Some(rgb),
                        bg: None,
                    }
                }
                (None, Some((r, g, b))) => {
                    let rgb = quantize(r, g, b, color_mode, color_boost, preserve_hue, cache);
                    Cell {
                        ch: blocks::LOWER_HALF,
                        fg: Some(rgb),
                        bg: None,
                    }
                }
                (Some((ur, ug, ub)), Some((lr, lg, lb))) => {
                    let upper_rgb = quantize(ur, ug, ub, color_mode, color_boost, preserve_hue, cache);
                    let lower_rgb = quantize(lr, lg, lb, color_mode, color_boost, preserve_hue, cache);
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
            ..Default::default()
        };
        let cells = import_image(&path, 4, 4, &opts).unwrap();
        assert_eq!(cells.len(), 4);
        assert_eq!(cells[0].len(), 4);

        // Every cell should have fg color (quantized red), full block char
        for row in &cells {
            for cell in row {
                assert_eq!(cell.ch, '\u{2588}');
                assert!(cell.fg.is_some());
                assert!(cell.bg.is_none());
                let fg = cell.fg.unwrap();
                assert!(fg.r > 100, "Red channel should be high, got {}", fg.r);
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
        let mid_has_content = cells[1].iter().any(|c| !c.is_empty())
            || cells[2].iter().any(|c| !c.is_empty());
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
            ..Default::default()
        };
        let cells = import_image(&path, 2, 2, &opts).unwrap();

        // Top-left should have color
        assert!(cells[0][0].fg.is_some());
        // Others should be empty (transparent)
        assert!(cells[0][1].fg.is_none());
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
            ..Default::default()
        };
        let cells = import_image(&path, 2, 2, &opts).unwrap();

        // Should have decoded (first frame = red)
        assert_eq!(cells.len(), 2);
        assert_eq!(cells[0].len(), 2);
        // Should have content from first frame
        let cell = &cells[0][0];
        assert!(cell.fg.is_some());
        let fg = cell.fg.unwrap();
        // First frame is red
        assert!(fg.r > 100, "Expected red first frame, got r={}", fg.r);
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
            ..Default::default()
        };
        let cells = import_image(&path, 2, 2, &opts).unwrap();

        let fg = cells[0][0].fg.unwrap();
        // 16-color quantized red should match one of the 16 ANSI colors
        let idx = cell::nearest_16(&Rgb::new(255, 0, 0));
        assert!(idx < 16, "Should quantize to ANSI 16 range");
        let expected = cell::color256_to_rgb(idx);
        assert_eq!(fg, expected);
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
            ..Default::default()
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

    #[test]
    fn test_import_default_truecolor() {
        // Verify default import stores full RGB (TrueColor mode preserves exact colors)
        let dir = std::env::temp_dir().join("kakukuma_test_default_tc");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("truecolor_test.png");

        // Create a 2x2 image with specific non-palette colors
        let pixels = vec![
            (137, 42, 201, 255), // purple not in xterm-256 exactly
            (42, 137, 201, 255),
            (201, 137, 42, 255),
            (42, 201, 137, 255),
        ];
        write_test_png(&path, 2, 2, &pixels);

        let opts = ImportOptions::default();
        assert_eq!(opts.color_mode, ImportColorMode::TrueColor);
        assert!(opts.normalize);
        assert!(opts.preserve_hue);

        let cells = import_image(&path, 2, 2, &opts).unwrap();
        // TrueColor mode should store exact RGB (after normalize stretch)
        // Key: the colors should NOT be quantized to the 256-color palette
        let cell = &cells[0][0];
        assert!(cell.bg.is_some() || cell.fg.is_some(),
            "Cell should have imported color");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_posterize_reduces_colors() {
        // An image with many colors should be reduced to N distinct colors
        let dir = std::env::temp_dir().join("kakukuma_test_posterize");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("gradient.png");

        // 8x8 gradient with many distinct colors
        let pixels: Vec<_> = (0..64).map(|i| {
            let r = (i * 4) as u8;
            let g = (255 - i * 3) as u8;
            let b = ((i * 7) % 256) as u8;
            (r, g, b, 255u8)
        }).collect();
        write_test_png(&path, 8, 8, &pixels);

        // Import with posterize=4
        let opts = ImportOptions {
            posterize: Some(4),
            color_mode: ImportColorMode::TrueColor,
            char_set: ImportCharSet::FullBlocks,
            ..Default::default()
        };
        let cells = import_image(&path, 8, 8, &opts).unwrap();

        // Count distinct bg colors
        let mut colors: std::collections::HashSet<(u8, u8, u8)> = std::collections::HashSet::new();
        for row in &cells {
            for cell in row {
                if let Some(fg) = &cell.fg {
                    colors.insert((fg.r, fg.g, fg.b));
                }
            }
        }
        assert!(colors.len() <= 4,
            "Posterize=4 should produce at most 4 distinct colors, got {}", colors.len());
        assert!(colors.len() >= 2,
            "Should have at least 2 colors from a gradient");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_import_no_normalize() {
        // Verify --no-normalize produces different output than normalize=true
        let dir = std::env::temp_dir().join("kakukuma_test_no_norm");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("dark_image.png");

        // Create a dark image where normalize would stretch brightness
        let pixels: Vec<_> = (0..16).map(|i| {
            let v = (i * 4) as u8; // range 0-60, very dark
            (v, v, v, 255)
        }).collect();
        write_test_png(&path, 4, 4, &pixels);

        let opts_norm = ImportOptions {
            normalize: true,
            color_mode: ImportColorMode::Color256,
            ..Default::default()
        };
        let opts_no_norm = ImportOptions {
            normalize: false,
            color_mode: ImportColorMode::Color256,
            ..Default::default()
        };

        let cells_norm = import_image(&path, 4, 4, &opts_norm).unwrap();
        let cells_no_norm = import_image(&path, 4, 4, &opts_no_norm).unwrap();

        // With normalization, dark pixels get stretched brighter
        // Without normalization, they stay dark
        // At least some cells should differ
        let mut differ = false;
        for y in 0..4 {
            for x in 0..4 {
                if cells_norm[y][x] != cells_no_norm[y][x] {
                    differ = true;
                }
            }
        }
        assert!(differ, "Normalize ON vs OFF should produce different results for a dark image");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
