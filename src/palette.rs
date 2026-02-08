use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::cell::Color256;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CustomPalette {
    pub name: String,
    pub colors: Vec<u8>,
}

/// List `.palette` files in the given directory.
pub fn list_palette_files(dir: &Path) -> Vec<String> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".palette") {
                    files.push(name.to_string());
                }
            }
        }
    }
    files.sort();
    files
}

/// Load a custom palette from a `.palette` JSON file.
pub fn load_palette(path: &Path) -> Result<CustomPalette, String> {
    let data = std::fs::read_to_string(path).map_err(|e| format!("Read error: {}", e))?;
    serde_json::from_str(&data).map_err(|e| format!("Parse error: {}", e))
}

/// Save a custom palette to a `.palette` JSON file.
pub fn save_palette(palette: &CustomPalette, path: &Path) -> Result<(), String> {
    let json = serde_json::to_string_pretty(palette).map_err(|e| format!("Serialize error: {}", e))?;
    std::fs::write(path, json).map_err(|e| format!("Write error: {}", e))
}

pub struct HueGroup {
    pub name: &'static str,
    pub colors: Vec<u8>,
}

/// Compute hue angle (0–359) from RGB, or None for grays.
fn rgb_hue(r: u8, g: u8, b: u8) -> Option<u16> {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = (max - min) as f32;
    if delta < 1.0 {
        return None; // Gray or near-gray
    }
    let (rf, gf, bf) = (r as f32, g as f32, b as f32);
    let hue = if max == r {
        60.0 * (((gf - bf) / delta) % 6.0)
    } else if max == g {
        60.0 * (((bf - rf) / delta) + 2.0)
    } else {
        60.0 * (((rf - gf) / delta) + 4.0)
    };
    let hue = if hue < 0.0 { hue + 360.0 } else { hue };
    Some(hue as u16 % 360)
}

/// Organize the 216 color cube (indices 16–231) into 8 hue groups.
/// Grays within the cube (where R==G==B or very low saturation) go to the
/// nearest chromatic group based on their index position.
pub fn build_hue_groups() -> Vec<HueGroup> {
    let mut reds = Vec::new();
    let mut oranges = Vec::new();
    let mut yellows = Vec::new();
    let mut greens = Vec::new();
    let mut cyans = Vec::new();
    let mut blues = Vec::new();
    let mut purples = Vec::new();
    let mut pinks = Vec::new();
    let mut neutrals = Vec::new();

    for idx in 16u8..=231 {
        let (r, g, b) = Color256(idx).to_rgb();
        match rgb_hue(r, g, b) {
            Some(h) => {
                match h {
                    0..=14 | 346..=359 => reds.push(idx),
                    15..=39 => oranges.push(idx),
                    40..=69 => yellows.push(idx),
                    70..=159 => greens.push(idx),
                    160..=199 => cyans.push(idx),
                    200..=259 => blues.push(idx),
                    260..=299 => purples.push(idx),
                    300..=345 => pinks.push(idx),
                    _ => neutrals.push(idx),
                }
            }
            None => {
                // Pure grays in the cube — assign to neutrals
                neutrals.push(idx);
            }
        }
    }

    // Distribute neutrals across groups proportionally to keep every index assigned
    // Simple approach: append them to the Reds group (they're R==G==B grays in the cube)
    reds.extend(neutrals);

    vec![
        HueGroup { name: "Reds", colors: reds },
        HueGroup { name: "Oranges", colors: oranges },
        HueGroup { name: "Yellows", colors: yellows },
        HueGroup { name: "Greens", colors: greens },
        HueGroup { name: "Cyans", colors: cyans },
        HueGroup { name: "Blues", colors: blues },
        HueGroup { name: "Purples", colors: purples },
        HueGroup { name: "Pinks", colors: pinks },
    ]
}

/// Build the flattened list of all browsable colors:
/// recent colors + standard 16 + hue groups + grays 232–255
pub fn all_palette_colors(
    recent: &[Color256],
    hue_groups: &[HueGroup],
) -> Vec<u8> {
    let mut colors = Vec::new();

    // Recent colors
    for c in recent {
        colors.push(c.0);
    }

    // Standard 16
    for i in 0..16u8 {
        colors.push(i);
    }

    // Hue groups
    for group in hue_groups {
        for &c in &group.colors {
            colors.push(c);
        }
    }

    // Grayscale 232–255
    for i in 232..=255u8 {
        colors.push(i);
    }

    colors
}

/// Convert RGB (0–255 each) to HSL. H in 0–359, S and L in 0–100.
pub fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (u16, u8, u8) {
    let rf = r as f32 / 255.0;
    let gf = g as f32 / 255.0;
    let bf = b as f32 / 255.0;

    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let l = (max + min) / 2.0;

    if (max - min).abs() < f32::EPSILON {
        // Achromatic
        return (0, 0, (l * 100.0).round() as u8);
    }

    let delta = max - min;
    let s = if l > 0.5 {
        delta / (2.0 - max - min)
    } else {
        delta / (max + min)
    };

    let h = if (max - rf).abs() < f32::EPSILON {
        let mut h = (gf - bf) / delta;
        if h < 0.0 {
            h += 6.0;
        }
        h
    } else if (max - gf).abs() < f32::EPSILON {
        (bf - rf) / delta + 2.0
    } else {
        (rf - gf) / delta + 4.0
    };

    let h = ((h * 60.0).round() as i32).rem_euclid(360) as u16;
    let s = (s * 100.0).round().min(100.0) as u8;
    let l = (l * 100.0).round().min(100.0) as u8;
    (h, s, l)
}

/// Convert HSL to RGB. H in 0–359, S and L in 0–100.
pub fn hsl_to_rgb(h: u16, s: u8, l: u8) -> (u8, u8, u8) {
    let s = s.min(100) as f32 / 100.0;
    let l = l.min(100) as f32 / 100.0;
    let h = (h % 360) as f32;

    if s < f32::EPSILON {
        let v = (l * 255.0).round() as u8;
        return (v, v, v);
    }

    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r1, g1, b1) = match h as u16 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    (
        ((r1 + m) * 255.0).round() as u8,
        ((g1 + m) * 255.0).round() as u8,
        ((b1 + m) * 255.0).round() as u8,
    )
}

/// Find the nearest xterm-256 color to an (R, G, B) value using Euclidean distance.
pub fn nearest_color(r: u8, g: u8, b: u8) -> Color256 {
    let mut best_idx: u8 = 0;
    let mut best_dist = u32::MAX;

    for i in 0u16..=255 {
        let (cr, cg, cb) = Color256(i as u8).to_rgb();
        let dr = r as i32 - cr as i32;
        let dg = g as i32 - cg as i32;
        let db = b as i32 - cb as i32;
        let dist = (dr * dr + dg * dg + db * db) as u32;
        if dist < best_dist {
            best_dist = dist;
            best_idx = i as u8;
        }
    }

    Color256(best_idx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_all_216_covered() {
        let groups = build_hue_groups();
        let mut seen: HashSet<u8> = HashSet::new();
        for group in &groups {
            for &c in &group.colors {
                assert!(c >= 16 && c <= 231, "Index {} out of cube range", c);
                assert!(seen.insert(c), "Duplicate index {}", c);
            }
        }
        assert_eq!(seen.len(), 216, "Expected 216 colors, got {}", seen.len());
    }

    #[test]
    fn test_no_missing_indices() {
        let groups = build_hue_groups();
        let mut all: Vec<u8> = Vec::new();
        for group in &groups {
            all.extend(&group.colors);
        }
        all.sort();
        let expected: Vec<u8> = (16..=231).collect();
        assert_eq!(all, expected);
    }

    #[test]
    fn test_eight_groups() {
        let groups = build_hue_groups();
        assert_eq!(groups.len(), 8);
        let names: Vec<&str> = groups.iter().map(|g| g.name).collect();
        assert!(names.contains(&"Reds"));
        assert!(names.contains(&"Blues"));
        assert!(names.contains(&"Greens"));
    }

    #[test]
    fn test_all_palette_colors_includes_everything() {
        let groups = build_hue_groups();
        let recent = vec![Color256(196), Color256(46)];
        let all = all_palette_colors(&recent, &groups);

        // Should contain recent + 16 standard + 216 cube + 24 grayscale
        assert!(all.len() >= 2 + 16 + 216 + 24);
        // Check recent are first
        assert_eq!(all[0], 196);
        assert_eq!(all[1], 46);
        // Check standard follow
        assert_eq!(all[2], 0);
        assert_eq!(all[17], 15);
    }

    #[test]
    fn test_rgb_hue_pure_red() {
        // Pure red (255, 0, 0) should be hue ~0
        let h = rgb_hue(255, 0, 0);
        assert!(h.is_some());
        assert!(h.unwrap() <= 15 || h.unwrap() >= 345);
    }

    #[test]
    fn test_rgb_hue_pure_green() {
        let h = rgb_hue(0, 255, 0);
        assert!(h.is_some());
        assert!(h.unwrap() >= 100 && h.unwrap() <= 140);
    }

    #[test]
    fn test_rgb_hue_gray_is_none() {
        assert!(rgb_hue(128, 128, 128).is_none());
        assert!(rgb_hue(0, 0, 0).is_none());
    }

    #[test]
    fn test_rgb_to_hsl_pure_red() {
        let (h, s, l) = rgb_to_hsl(255, 0, 0);
        assert_eq!(h, 0);
        assert_eq!(s, 100);
        assert_eq!(l, 50);
    }

    #[test]
    fn test_rgb_to_hsl_pure_green() {
        let (h, s, l) = rgb_to_hsl(0, 255, 0);
        assert_eq!(h, 120);
        assert_eq!(s, 100);
        assert_eq!(l, 50);
    }

    #[test]
    fn test_rgb_to_hsl_pure_blue() {
        let (h, s, l) = rgb_to_hsl(0, 0, 255);
        assert_eq!(h, 240);
        assert_eq!(s, 100);
        assert_eq!(l, 50);
    }

    #[test]
    fn test_rgb_to_hsl_white() {
        let (h, s, l) = rgb_to_hsl(255, 255, 255);
        assert_eq!(h, 0);
        assert_eq!(s, 0);
        assert_eq!(l, 100);
    }

    #[test]
    fn test_rgb_to_hsl_black() {
        let (h, s, l) = rgb_to_hsl(0, 0, 0);
        assert_eq!(h, 0);
        assert_eq!(s, 0);
        assert_eq!(l, 0);
    }

    #[test]
    fn test_hsl_to_rgb_pure_red() {
        assert_eq!(hsl_to_rgb(0, 100, 50), (255, 0, 0));
    }

    #[test]
    fn test_hsl_to_rgb_pure_green() {
        assert_eq!(hsl_to_rgb(120, 100, 50), (0, 255, 0));
    }

    #[test]
    fn test_hsl_to_rgb_gray() {
        let (r, g, b) = hsl_to_rgb(0, 0, 50);
        assert_eq!(r, g);
        assert_eq!(g, b);
        assert!((r as i16 - 128).abs() <= 1);
    }

    #[test]
    fn test_hsl_roundtrip_pure_colors() {
        for &(r, g, b) in &[(255u8, 0, 0), (0, 255, 0), (0, 0, 255), (255, 255, 0), (0, 255, 255)] {
            let (h, s, l) = rgb_to_hsl(r, g, b);
            let (r2, g2, b2) = hsl_to_rgb(h, s, l);
            assert!((r as i16 - r2 as i16).abs() <= 1, "R mismatch for ({},{},{}): got ({},{},{})", r, g, b, r2, g2, b2);
            assert!((g as i16 - g2 as i16).abs() <= 1, "G mismatch for ({},{},{}): got ({},{},{})", r, g, b, r2, g2, b2);
            assert!((b as i16 - b2 as i16).abs() <= 1, "B mismatch for ({},{},{}): got ({},{},{})", r, g, b, r2, g2, b2);
        }
    }

    #[test]
    fn test_nearest_color_pure_red() {
        // Pure red (255, 0, 0) should map to index 196 (cube) or 9 (bright red)
        let c = nearest_color(255, 0, 0);
        assert!(c == Color256(196) || c == Color256(9), "Got {:?}", c);
    }

    #[test]
    fn test_nearest_color_pure_green() {
        let c = nearest_color(0, 255, 0);
        assert!(c == Color256(46) || c == Color256(10), "Got {:?}", c);
    }

    #[test]
    fn test_nearest_color_pure_blue() {
        let c = nearest_color(0, 0, 255);
        assert!(c == Color256(21) || c == Color256(12), "Got {:?}", c);
    }

    #[test]
    fn test_nearest_color_black() {
        assert_eq!(nearest_color(0, 0, 0), Color256(0));
    }

    #[test]
    fn test_nearest_color_white() {
        let c = nearest_color(255, 255, 255);
        assert!(c == Color256(15) || c == Color256(231), "Got {:?}", c);
    }

    #[test]
    fn test_custom_palette_save_load_roundtrip() {
        let palette = CustomPalette {
            name: "Test Forest".to_string(),
            colors: vec![22, 28, 34, 40, 46],
        };
        let dir = std::env::temp_dir();
        let path = dir.join("kaku_test_roundtrip.palette");
        save_palette(&palette, &path).unwrap();

        let loaded = load_palette(&path).unwrap();
        assert_eq!(loaded.name, "Test Forest");
        assert_eq!(loaded.colors, vec![22, 28, 34, 40, 46]);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_list_palette_files() {
        let dir = std::env::temp_dir().join("kaku_test_list_palettes");
        let _ = std::fs::create_dir_all(&dir);

        // Create test files
        std::fs::write(dir.join("forest.palette"), "{}").unwrap();
        std::fs::write(dir.join("ocean.palette"), "{}").unwrap();
        std::fs::write(dir.join("not_a_palette.txt"), "nope").unwrap();

        let files = list_palette_files(&dir);
        assert!(files.contains(&"forest.palette".to_string()));
        assert!(files.contains(&"ocean.palette".to_string()));
        assert!(!files.contains(&"not_a_palette.txt".to_string()));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
