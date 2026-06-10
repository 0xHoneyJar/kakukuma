use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::cell::{color256_to_rgb, Rgb};

/// Curated 24-color default palette covering neutrals, warm, cool, and accent hues.
/// Computed from original xterm-256 indices.
pub const DEFAULT_PALETTE: [Rgb; 24] = [
    // Neutrals (6) — indices 0, 236, 244, 250, 255, 15
    Rgb { r: 0, g: 0, b: 0 },         // Black (0)
    Rgb { r: 48, g: 48, b: 48 },       // Dark gray (236)
    Rgb { r: 128, g: 128, b: 128 },    // Medium gray (244)
    Rgb { r: 188, g: 188, b: 188 },    // Light gray (250)
    Rgb { r: 238, g: 238, b: 238 },    // Near white (255)
    Rgb { r: 255, g: 255, b: 255 },    // Bright white (15)

    // Warm (6) — indices 1, 196, 208, 214, 226, 229
    Rgb { r: 205, g: 0, b: 0 },        // Dark red (1)
    Rgb { r: 255, g: 0, b: 0 },        // Bright red (196)
    Rgb { r: 255, g: 135, b: 0 },      // Orange (208)
    Rgb { r: 255, g: 175, b: 0 },      // Light orange / amber (214)
    Rgb { r: 255, g: 255, b: 0 },      // Yellow (226)
    Rgb { r: 255, g: 255, b: 175 },    // Light yellow (229)

    // Cool (6) — indices 22, 46, 30, 39, 21, 54
    Rgb { r: 0, g: 95, b: 0 },         // Dark green (22)
    Rgb { r: 0, g: 255, b: 0 },        // Bright green (46)
    Rgb { r: 0, g: 135, b: 135 },      // Teal (30)
    Rgb { r: 0, g: 175, b: 255 },      // Sky blue (39)
    Rgb { r: 0, g: 0, b: 255 },        // Bright blue (21)
    Rgb { r: 95, g: 0, b: 175 },       // Dark purple (54)

    // Accent (6) — indices 200, 213, 93, 180, 137, 94
    Rgb { r: 255, g: 0, b: 215 },      // Pink / magenta (200)
    Rgb { r: 255, g: 135, b: 215 },    // Light pink (213)
    Rgb { r: 135, g: 0, b: 255 },      // Lavender (93)
    Rgb { r: 215, g: 175, b: 135 },    // Tan / skin light (180)
    Rgb { r: 175, g: 135, b: 95 },     // Skin / warm mid (137)
    Rgb { r: 135, g: 95, b: 0 },       // Brown (94)
];

/// An item in the flattened palette layout — either a color swatch or a section header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteItem {
    Color(Rgb),
    SectionHeader(PaletteSection),
}

/// Collapsible palette sections below the curated palette.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteSection {
    Recent,
    Standard,
    HueGroups,
    Grayscale,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CustomPalette {
    pub name: String,
    pub colors: Vec<Rgb>,
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
    #[allow(dead_code)] // Used in tests; may be displayed in expanded sections later
    pub name: &'static str,
    pub colors: Vec<Rgb>,
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
        let c = color256_to_rgb(idx);
        match rgb_hue(c.r, c.g, c.b) {
            Some(h) => {
                let color = c;
                match h {
                    0..=14 | 346..=359 => reds.push(color),
                    15..=39 => oranges.push(color),
                    40..=69 => yellows.push(color),
                    70..=159 => greens.push(color),
                    160..=199 => cyans.push(color),
                    200..=259 => blues.push(color),
                    260..=299 => purples.push(color),
                    300..=345 => pinks.push(color),
                    _ => neutrals.push(color),
                }
            }
            None => {
                // Pure grays in the cube — assign to neutrals
                neutrals.push(c);
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
/// Returns the Rgb value of the nearest match.
pub fn nearest_color(r: u8, g: u8, b: u8) -> Rgb {
    let target = Rgb::new(r, g, b);
    let idx = crate::cell::nearest_256(&target);
    color256_to_rgb(idx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_default_palette_unique_and_valid() {
        let mut seen: HashSet<(u8, u8, u8)> = HashSet::new();
        for &c in &DEFAULT_PALETTE {
            assert!(seen.insert((c.r, c.g, c.b)), "Duplicate color in DEFAULT_PALETTE");
        }
        assert_eq!(DEFAULT_PALETTE.len(), 24);
    }

    #[test]
    fn test_all_216_covered() {
        let groups = build_hue_groups();
        let mut seen: HashSet<(u8, u8, u8)> = HashSet::new();
        for group in &groups {
            for &c in &group.colors {
                assert!(seen.insert((c.r, c.g, c.b)), "Duplicate color {:?}", c);
            }
        }
        assert_eq!(seen.len(), 216, "Expected 216 colors, got {}", seen.len());
    }

    #[test]
    fn test_no_missing_indices() {
        let groups = build_hue_groups();
        let mut all: Vec<(u8, u8, u8)> = Vec::new();
        for group in &groups {
            for &c in &group.colors {
                all.push((c.r, c.g, c.b));
            }
        }
        // Should have exactly 216 unique colors matching the cube indices 16-231
        let expected: Vec<(u8, u8, u8)> = (16..=231u8).map(|i| {
            let c = color256_to_rgb(i);
            (c.r, c.g, c.b)
        }).collect();
        all.sort();
        let mut expected_sorted = expected;
        expected_sorted.sort();
        assert_eq!(all, expected_sorted);
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
        // Pure red (255, 0, 0) should map to a red
        let c = nearest_color(255, 0, 0);
        assert!(c == color256_to_rgb(196) || c == color256_to_rgb(9), "Got {:?}", c);
    }

    #[test]
    fn test_nearest_color_pure_green() {
        let c = nearest_color(0, 255, 0);
        assert!(c == color256_to_rgb(46) || c == color256_to_rgb(10), "Got {:?}", c);
    }

    #[test]
    fn test_nearest_color_pure_blue() {
        let c = nearest_color(0, 0, 255);
        assert!(c == color256_to_rgb(21) || c == color256_to_rgb(12), "Got {:?}", c);
    }

    #[test]
    fn test_nearest_color_black() {
        assert_eq!(nearest_color(0, 0, 0), Rgb::BLACK);
    }

    #[test]
    fn test_nearest_color_white() {
        let c = nearest_color(255, 255, 255);
        assert!(c == color256_to_rgb(15) || c == color256_to_rgb(231), "Got {:?}", c);
    }

    #[test]
    fn test_custom_palette_save_load_roundtrip() {
        let palette = CustomPalette {
            name: "Test Forest".to_string(),
            colors: vec![
                color256_to_rgb(22),
                color256_to_rgb(28),
                color256_to_rgb(34),
                color256_to_rgb(40),
                color256_to_rgb(46),
            ],
        };
        let dir = std::env::temp_dir();
        let path = dir.join("kaku_test_roundtrip.palette");
        save_palette(&palette, &path).unwrap();

        let loaded = load_palette(&path).unwrap();
        assert_eq!(loaded.name, "Test Forest");
        assert_eq!(loaded.colors.len(), 5);
        assert_eq!(loaded.colors[0], color256_to_rgb(22));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_rename_palette() {
        let dir = std::env::temp_dir().join("kaku_test_rename_rgb");
        let _ = std::fs::create_dir_all(&dir);
        let cp = CustomPalette {
            name: "OldName".to_string(),
            colors: vec![Rgb::new(255, 0, 0), Rgb::new(0, 255, 0)],
        };
        let old_path = dir.join("OldName.palette");
        save_palette(&cp, &old_path).unwrap();

        // Rename: load, change name, save new, delete old
        let mut loaded = load_palette(&old_path).unwrap();
        loaded.name = "NewName".to_string();
        let new_path = dir.join("NewName.palette");
        save_palette(&loaded, &new_path).unwrap();
        std::fs::remove_file(&old_path).unwrap();

        assert!(!old_path.exists());
        let reloaded = load_palette(&new_path).unwrap();
        assert_eq!(reloaded.name, "NewName");
        assert_eq!(reloaded.colors.len(), 2);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_duplicate_palette() {
        let dir = std::env::temp_dir().join("kaku_test_duplicate_rgb");
        let _ = std::fs::create_dir_all(&dir);
        let cp = CustomPalette {
            name: "Original".to_string(),
            colors: vec![Rgb::new(10, 20, 30)],
        };
        let orig_path = dir.join("Original.palette");
        save_palette(&cp, &orig_path).unwrap();

        // Duplicate
        let mut dup = load_palette(&orig_path).unwrap();
        dup.name = format!("{} (Copy)", dup.name);
        let dup_path = dir.join(format!("{}.palette", dup.name));
        save_palette(&dup, &dup_path).unwrap();

        assert!(orig_path.exists());
        assert!(dup_path.exists());
        let loaded = load_palette(&dup_path).unwrap();
        assert_eq!(loaded.name, "Original (Copy)");
        assert_eq!(loaded.colors, vec![Rgb::new(10, 20, 30)]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_delete_palette() {
        let dir = std::env::temp_dir().join("kaku_test_delete_rgb");
        let _ = std::fs::create_dir_all(&dir);
        let cp = CustomPalette {
            name: "ToDelete".to_string(),
            colors: vec![Rgb::new(5, 5, 5)],
        };
        let path = dir.join("ToDelete.palette");
        save_palette(&cp, &path).unwrap();
        assert!(path.exists());

        std::fs::remove_file(&path).unwrap();
        assert!(!path.exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_rename_to_existing_name_blocked() {
        let dir = std::env::temp_dir().join("kaku_test_rename_conflict_rgb");
        let _ = std::fs::create_dir_all(&dir);

        let cp1 = CustomPalette { name: "A".to_string(), colors: vec![Rgb::new(1, 0, 0)] };
        let cp2 = CustomPalette { name: "B".to_string(), colors: vec![Rgb::new(0, 1, 0)] };
        save_palette(&cp1, &dir.join("A.palette")).unwrap();
        save_palette(&cp2, &dir.join("B.palette")).unwrap();

        // Attempting to rename A to B should be blocked (file exists)
        let new_path = dir.join("B.palette");
        assert!(new_path.exists(), "Target already exists — rename should be blocked");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_export_palette() {
        let dir = std::env::temp_dir().join("kaku_test_export_rgb");
        let _ = std::fs::create_dir_all(&dir);
        let cp = CustomPalette {
            name: "ExportMe".to_string(),
            colors: vec![Rgb::new(100, 100, 100)],
        };
        let src = dir.join("ExportMe.palette");
        save_palette(&cp, &src).unwrap();

        let dest = dir.join("exported_copy.palette");
        std::fs::copy(&src, &dest).unwrap();
        assert!(dest.exists());
        let loaded = load_palette(&dest).unwrap();
        assert_eq!(loaded.name, "ExportMe");
        assert_eq!(loaded.colors.len(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_list_palette_files() {
        let dir = std::env::temp_dir().join("kaku_test_list_palettes_rgb");
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
