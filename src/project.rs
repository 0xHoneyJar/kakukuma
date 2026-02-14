use serde::{Deserialize, Serialize};

use crate::canvas::Canvas;
use crate::cell::Rgb;
use crate::symmetry::SymmetryMode;

#[derive(Serialize, Deserialize)]
pub struct Project {
    pub version: u32,
    pub name: String,
    pub created_at: String,
    pub modified_at: String,
    pub color: Rgb,
    pub symmetry: SymmetryMode,
    pub canvas: Canvas,
}

impl Project {
    pub fn new(name: &str, canvas: Canvas, color: Rgb, sym: SymmetryMode) -> Self {
        let now = now_iso8601();
        Project {
            version: 5,
            name: name.to_string(),
            created_at: now.clone(),
            modified_at: now,
            color,
            symmetry: sym,
            canvas,
        }
    }

    pub fn save_to_file(&mut self, path: &std::path::Path) -> Result<(), String> {
        self.modified_at = now_iso8601();
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Serialize error: {}", e))?;
        std::fs::write(path, json)
            .map_err(|e| format!("Write error: {}", e))
    }

    pub fn load_from_file(path: &std::path::Path) -> Result<Self, String> {
        let data = std::fs::read_to_string(path)
            .map_err(|e| format!("Read error: {}", e))?;
        let project: Project = serde_json::from_str(&data)
            .map_err(|e| format!("Parse error: {}", e))?;
        // Accept v1 (legacy 16-color), v2 (256-color), v3 (dynamic canvas), v4 (generic char), v5 (RGB)
        if project.version > 5 {
            return Err(format!(
                "File version {} is newer than supported (v5)",
                project.version
            ));
        }
        Ok(project)
    }
}

/// List .kaku files in the given directory, sorted by name.
pub fn list_kaku_files(dir: &std::path::Path) -> Vec<String> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("kaku") {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    files.push(name.to_string());
                }
            }
        }
    }
    files.sort();
    files
}

/// Find autosave files in the given directory.
pub fn find_autosave(dir: &std::path::Path) -> Option<String> {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.ends_with(".kaku.autosave") {
                    return Some(name.to_string());
                }
            }
        }
    }
    None
}

pub(crate) fn now_iso8601() -> String {
    // Simple UTC timestamp without external crate
    use std::time::SystemTime;
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    // Rough conversion - good enough for a timestamp
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;

    // Calculate date from days since epoch (1970-01-01)
    let (year, month, day) = days_to_date(days);

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

fn days_to_date(days: u64) -> (u64, u64, u64) {
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::Canvas;
    use crate::cell::{blocks, Cell, Rgb, color256_to_rgb};

    #[test]
    fn test_save_load_roundtrip() {
        let mut canvas = Canvas::new();
        canvas.set(5, 10, Cell {
            ch: blocks::FULL,
            fg: Some(color256_to_rgb(1)),
            bg: Some(color256_to_rgb(4)),
        });

        let mut project = Project::new(
            "test-project",
            canvas,
            color256_to_rgb(2),
            SymmetryMode::Horizontal,
        );

        let dir = std::env::temp_dir();
        let path = dir.join("kaku_test_roundtrip_v5.kaku");
        project.save_to_file(&path).unwrap();

        let loaded = Project::load_from_file(&path).unwrap();
        assert_eq!(loaded.name, "test-project");
        assert_eq!(loaded.color, color256_to_rgb(2));
        assert_eq!(loaded.symmetry, SymmetryMode::Horizontal);
        assert_eq!(loaded.version, 5);
        assert_eq!(
            loaded.canvas.get(5, 10),
            Some(Cell {
                ch: blocks::FULL,
                fg: Some(color256_to_rgb(1)),
                bg: Some(color256_to_rgb(4)),
            })
        );
        assert_eq!(loaded.canvas.get(0, 0), Some(Cell::default()));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_save_load_rgb_color() {
        let mut canvas = Canvas::new();
        canvas.set(0, 0, Cell {
            ch: blocks::FULL,
            fg: Some(Rgb::new(255, 0, 0)),
            bg: Some(Rgb::new(0, 0, 255)),
        });

        let mut project = Project::new(
            "color-test",
            canvas,
            Rgb::new(100, 200, 50),
            SymmetryMode::Off,
        );

        let dir = std::env::temp_dir();
        let path = dir.join("kaku_test_rgb_color.kaku");
        project.save_to_file(&path).unwrap();

        let loaded = Project::load_from_file(&path).unwrap();
        assert_eq!(loaded.color, Rgb::new(100, 200, 50));
        assert_eq!(
            loaded.canvas.get(0, 0),
            Some(Cell {
                ch: blocks::FULL,
                fg: Some(Rgb::new(255, 0, 0)),
                bg: Some(Rgb::new(0, 0, 255)),
            })
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_load_legacy_v1_file() {
        // Build a valid v1-style project with string color name,
        // then patch the JSON to use the legacy "Green" format.
        let canvas = Canvas::new();
        let mut project = Project::new("legacy-art", canvas, color256_to_rgb(2), crate::symmetry::SymmetryMode::Off);
        project.version = 1;

        let dir = std::env::temp_dir();
        let path = dir.join("kaku_test_legacy_v1_rgb.kaku");
        project.save_to_file(&path).unwrap();

        // Patch the saved JSON: replace RGB array with legacy string
        let json = std::fs::read_to_string(&path).unwrap();
        let patched = json.replacen(
            "\"color\": [\n    0,\n    205,\n    0\n  ]",
            "\"color\": \"Green\"",
            1,
        );
        std::fs::write(&path, patched).unwrap();

        let loaded = Project::load_from_file(&path).unwrap();
        assert_eq!(loaded.name, "legacy-art");
        assert_eq!(loaded.color, color256_to_rgb(2)); // Green → index 2

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_load_invalid_file() {
        let dir = std::env::temp_dir();
        let path = dir.join("kaku_test_invalid.kaku");
        std::fs::write(&path, "not valid json").unwrap();

        let result = Project::load_from_file(&path);
        assert!(result.is_err());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_list_kaku_files() {
        let dir = std::env::temp_dir().join("kaku_test_list");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("art1.kaku"), "{}").unwrap();
        std::fs::write(dir.join("art2.kaku"), "{}").unwrap();
        std::fs::write(dir.join("readme.txt"), "nope").unwrap();

        let files = list_kaku_files(&dir);
        assert_eq!(files.len(), 2);
        assert!(files.contains(&"art1.kaku".to_string()));
        assert!(files.contains(&"art2.kaku".to_string()));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_find_autosave() {
        let dir = std::env::temp_dir().join("kaku_test_autosave");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("myart.kaku.autosave"), "{}").unwrap();

        let found = find_autosave(&dir);
        assert!(found.is_some());
        assert!(found.unwrap().ends_with(".kaku.autosave"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    // --- Cycle 15 QA: File I/O roundtrip tests ---

    #[test]
    fn test_roundtrip_shade_chars() {
        let mut canvas = Canvas::new();
        let shades = [blocks::SHADE_LIGHT, blocks::SHADE_MEDIUM, blocks::SHADE_DARK];
        for (i, &ch) in shades.iter().enumerate() {
            canvas.set(i, 0, Cell {
                ch,
                fg: Some(Rgb::new(200, 100, 50)),
                bg: None,
            });
        }

        let mut project = Project::new("shade-test", canvas, Rgb::WHITE, SymmetryMode::Off);
        let dir = std::env::temp_dir();
        let path = dir.join("kaku_test_roundtrip_shades.kaku");
        project.save_to_file(&path).unwrap();

        let loaded = Project::load_from_file(&path).unwrap();
        for (i, &ch) in shades.iter().enumerate() {
            let cell = loaded.canvas.get(i, 0).unwrap();
            assert_eq!(cell.ch, ch, "Shade char at position {} mismatch", i);
            assert_eq!(cell.fg, Some(Rgb::new(200, 100, 50)));
        }

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_roundtrip_fractional_fills() {
        let mut canvas = Canvas::new();
        let fills = [
            blocks::LOWER_1_8, blocks::LOWER_1_4, blocks::LOWER_3_8,
            blocks::LEFT_7_8, blocks::LEFT_3_4, blocks::LEFT_1_8,
        ];
        for (i, &ch) in fills.iter().enumerate() {
            canvas.set(i, 0, Cell {
                ch,
                fg: Some(Rgb::new(0, 255, 0)),
                bg: None,
            });
        }

        let mut project = Project::new("fill-test", canvas, Rgb::WHITE, SymmetryMode::Off);
        let dir = std::env::temp_dir();
        let path = dir.join("kaku_test_roundtrip_fills.kaku");
        project.save_to_file(&path).unwrap();

        let loaded = Project::load_from_file(&path).unwrap();
        for (i, &ch) in fills.iter().enumerate() {
            let cell = loaded.canvas.get(i, 0).unwrap();
            assert_eq!(cell.ch, ch, "Fill char at position {} mismatch", i);
        }

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_roundtrip_all_block_types() {
        let mut canvas = Canvas::new();
        for (i, &ch) in blocks::ALL.iter().enumerate() {
            canvas.set(i, 0, Cell {
                ch,
                fg: Some(Rgb::new(128, 64, 32)),
                bg: if i % 2 == 0 { Some(Rgb::new(10, 20, 30)) } else { None },
            });
        }

        let mut project = Project::new("all-blocks", canvas, Rgb::WHITE, SymmetryMode::Off);
        let dir = std::env::temp_dir();
        let path = dir.join("kaku_test_roundtrip_all_blocks.kaku");
        project.save_to_file(&path).unwrap();

        let loaded = Project::load_from_file(&path).unwrap();
        for (i, &ch) in blocks::ALL.iter().enumerate() {
            let cell = loaded.canvas.get(i, 0).unwrap();
            assert_eq!(cell.ch, ch, "Block {} at position {} mismatch", ch, i);
            assert_eq!(cell.fg, Some(Rgb::new(128, 64, 32)));
            if i % 2 == 0 {
                assert_eq!(cell.bg, Some(Rgb::new(10, 20, 30)));
            } else {
                assert_eq!(cell.bg, None);
            }
        }

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_roundtrip_colors_exact() {
        let mut canvas = Canvas::new();
        let colors = [
            (Rgb::new(0, 0, 0), Rgb::new(255, 255, 255)),
            (Rgb::new(127, 127, 127), Rgb::new(1, 2, 3)),
            (Rgb::new(255, 0, 128), Rgb::new(0, 128, 255)),
        ];
        for (i, (fg, bg)) in colors.iter().enumerate() {
            canvas.set(i, 0, Cell {
                ch: blocks::FULL,
                fg: Some(*fg),
                bg: Some(*bg),
            });
        }

        let mut project = Project::new("color-exact", canvas, Rgb::new(42, 43, 44), SymmetryMode::Off);
        let dir = std::env::temp_dir();
        let path = dir.join("kaku_test_roundtrip_colors_exact.kaku");
        project.save_to_file(&path).unwrap();

        let loaded = Project::load_from_file(&path).unwrap();
        assert_eq!(loaded.color, Rgb::new(42, 43, 44));
        for (i, (fg, bg)) in colors.iter().enumerate() {
            let cell = loaded.canvas.get(i, 0).unwrap();
            assert_eq!(cell.fg, Some(*fg), "fg mismatch at {}", i);
            assert_eq!(cell.bg, Some(*bg), "bg mismatch at {}", i);
        }

        let _ = std::fs::remove_file(&path);
    }
}
