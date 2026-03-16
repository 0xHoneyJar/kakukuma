//! Integration tests verifying the kakukuma library API surface.
//! These tests confirm that downstream consumers can use the public API
//! without depending on binary-internal modules.

use kakukuma::canvas::{Canvas, DEFAULT_WIDTH, DEFAULT_HEIGHT};
use kakukuma::cell::{self, blocks, Cell, Rgb, nearest_256, nearest_16, resolve_half_block};
use kakukuma::export;
use kakukuma::project::Project;
use kakukuma::symmetry::SymmetryMode;

#[test]
fn canvas_create_default() {
    let canvas = Canvas::new();
    assert_eq!(canvas.width, DEFAULT_WIDTH);
    assert_eq!(canvas.height, DEFAULT_HEIGHT);
}

#[test]
fn canvas_create_with_size_and_resize() {
    let mut canvas = Canvas::new_with_size(16, 16);
    assert_eq!(canvas.width, 16);
    assert_eq!(canvas.height, 16);

    canvas.resize(32, 24);
    assert_eq!(canvas.width, 32);
    assert_eq!(canvas.height, 24);
}

#[test]
fn cell_default_is_transparent() {
    let cell = Cell::default();
    assert_eq!(cell.ch, ' ');
    // Default fg is white (not transparent)
    assert_eq!(cell.fg, Some(Rgb::WHITE));
    assert!(cell.bg.is_none());
}

#[test]
fn rgb_to_ratatui_returns_indexed() {
    let color = Rgb { r: 255, g: 0, b: 0 };
    let ratatui_color = color.to_ratatui();
    // Must be Color::Indexed, never Color::Rgb
    match ratatui_color {
        ratatui::style::Color::Indexed(_) => {}
        other => panic!("Expected Color::Indexed, got {:?}", other),
    }
}

#[test]
fn nearest_256_and_16_are_deterministic() {
    let red = Rgb { r: 255, g: 0, b: 0 };
    let idx256 = nearest_256(&red);
    let idx16 = nearest_16(&red);

    // Same input always gives same output (deterministic)
    assert_eq!(nearest_256(&red), idx256);
    assert_eq!(nearest_16(&red), idx16);

    // Both should return valid color indices
    assert!(idx16 < 16);
    // 256-color index in valid range
    assert!(idx256 > 0);
}

#[test]
fn half_block_constants_accessible() {
    assert_eq!(blocks::UPPER_HALF, '\u{2580}');
    assert_eq!(blocks::LOWER_HALF, '\u{2584}');
    assert_eq!(blocks::FULL, '\u{2588}');
}

#[test]
fn resolve_half_block_on_non_block_returns_none() {
    let cell = Cell { ch: 'A', fg: Some(Rgb { r: 255, g: 0, b: 0 }), bg: None };
    assert!(resolve_half_block(&cell).is_none());
}

#[test]
fn project_roundtrip_json() {
    let mut canvas = Canvas::new_with_size(8, 8);
    canvas.set(0, 0, Cell {
        ch: blocks::LOWER_HALF,
        fg: Some(Rgb { r: 255, g: 0, b: 0 }),
        bg: Some(Rgb { r: 0, g: 0, b: 255 }),
    });

    let project = Project::new("test", canvas, Rgb { r: 255, g: 255, b: 255 }, SymmetryMode::Off);
    let json = serde_json::to_string(&project).unwrap();
    let restored: Project = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.canvas.width, 8);
    assert_eq!(restored.canvas.height, 8);
    let cell = restored.canvas.get(0, 0).unwrap();
    assert_eq!(cell.fg, Some(Rgb { r: 255, g: 0, b: 0 }));
    assert_eq!(cell.bg, Some(Rgb { r: 0, g: 0, b: 255 }));
}

#[test]
fn export_plain_text_from_canvas() {
    let mut canvas = Canvas::new_with_size(8, 8);
    canvas.set(0, 0, Cell { ch: 'H', fg: None, bg: None });
    canvas.set(1, 0, Cell { ch: 'i', fg: None, bg: None });

    let text = export::to_plain_text(&canvas);
    assert!(text.starts_with("Hi"));
}

#[test]
fn symmetry_modes_accessible() {
    let _off = SymmetryMode::Off;
    let _h = SymmetryMode::Horizontal;
    let _v = SymmetryMode::Vertical;
    let _q = SymmetryMode::Quad;
}

#[test]
fn cell_nearest_16_accessible() {
    // Verify cell module functions are publicly accessible
    let white = Rgb { r: 255, g: 255, b: 255 };
    let idx = cell::nearest_16(&white);
    // ANSI 15 is bright white
    assert_eq!(idx, 15);
}
