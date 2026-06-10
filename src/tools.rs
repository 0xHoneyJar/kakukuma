use crate::canvas::Canvas;
use crate::cell::{Cell, Rgb};
use crate::history::CellMutation;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ToolKind {
    Pencil,
    Eraser,
    Line,
    Rectangle,
    Fill,
    Eyedropper,
}

impl ToolKind {
    pub fn name(self) -> &'static str {
        match self {
            ToolKind::Pencil => "Pencil",
            ToolKind::Eraser => "Eraser",
            ToolKind::Line => "Line",
            ToolKind::Rectangle => "Rect",
            ToolKind::Fill => "Fill",
            ToolKind::Eyedropper => "Pick",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            ToolKind::Pencil => "\u{270F}",    // ✏
            ToolKind::Eraser => "\u{25FB}",    // ◻
            ToolKind::Line => "\u{2571}",      // ╱
            ToolKind::Rectangle => "\u{25AD}", // ▭
            ToolKind::Fill => "\u{25C9}",      // ◉
            ToolKind::Eyedropper => "\u{25C8}", // ◈
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            ToolKind::Pencil => "P",
            ToolKind::Eraser => "E",
            ToolKind::Line => "L",
            ToolKind::Rectangle => "R",
            ToolKind::Fill => "F",
            ToolKind::Eyedropper => "I",
        }
    }

    pub const ALL: [ToolKind; 6] = [
        ToolKind::Pencil,
        ToolKind::Eraser,
        ToolKind::Line,
        ToolKind::Rectangle,
        ToolKind::Fill,
        ToolKind::Eyedropper,
    ];
}

#[derive(Clone, Debug)]
pub enum ToolState {
    Idle,
    LineStart { x: usize, y: usize },
    RectStart { x: usize, y: usize },
}

/// Place a single cell (pencil).
pub fn pencil(
    canvas: &Canvas,
    x: usize,
    y: usize,
    ch: char,
    fg: Option<Rgb>,
    bg: Option<Rgb>,
) -> Vec<CellMutation> {
    if let Some(old) = canvas.get(x, y) {
        let new = Cell { ch, fg, bg };
        if old != new {
            vec![CellMutation { x, y, old, new }]
        } else {
            vec![]
        }
    } else {
        vec![]
    }
}

/// Erase a cell (set to empty with default bg).
pub fn eraser(canvas: &Canvas, x: usize, y: usize) -> Vec<CellMutation> {
    if let Some(old) = canvas.get(x, y) {
        let new = Cell::default();
        if old != new {
            vec![CellMutation { x, y, old, new }]
        } else {
            vec![]
        }
    } else {
        vec![]
    }
}

/// Bresenham's line algorithm. Returns list of (x, y) points.
pub fn bresenham_line(x0: usize, y0: usize, x1: usize, y1: usize) -> Vec<(usize, usize)> {
    let mut points = Vec::new();
    let (mut x0, mut y0) = (x0 as isize, y0 as isize);
    let (x1, y1) = (x1 as isize, y1 as isize);

    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        points.push((x0 as usize, y0 as usize));
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }

    points
}

/// Draw a line from (x0,y0) to (x1,y1).
#[allow(clippy::too_many_arguments)]
pub fn line(
    canvas: &Canvas,
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
    ch: char,
    fg: Option<Rgb>,
    bg: Option<Rgb>,
) -> Vec<CellMutation> {
    let points = bresenham_line(x0, y0, x1, y1);
    let new = Cell { ch, fg, bg };
    let mut mutations = Vec::new();
    for (x, y) in points {
        if let Some(old) = canvas.get(x, y) {
            if old != new {
                mutations.push(CellMutation { x, y, old, new });
            }
        }
    }
    mutations
}

/// Draw a rectangle outline from (x0,y0) to (x1,y1).
#[allow(clippy::too_many_arguments)]
pub fn rectangle(
    canvas: &Canvas,
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
    ch: char,
    fg: Option<Rgb>,
    bg: Option<Rgb>,
    filled: bool,
) -> Vec<CellMutation> {
    let min_x = x0.min(x1);
    let max_x = x0.max(x1);
    let min_y = y0.min(y1);
    let max_y = y0.max(y1);
    let new = Cell { ch, fg, bg };
    let mut mutations = Vec::new();

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let is_border = x == min_x || x == max_x || y == min_y || y == max_y;
            if filled || is_border {
                if let Some(old) = canvas.get(x, y) {
                    if old != new {
                        mutations.push(CellMutation { x, y, old, new });
                    }
                }
            }
        }
    }
    mutations
}

/// Iterative flood fill from (start_x, start_y).
pub fn flood_fill(
    canvas: &Canvas,
    start_x: usize,
    start_y: usize,
    ch: char,
    fg: Option<Rgb>,
    bg: Option<Rgb>,
) -> Vec<CellMutation> {
    let target = match canvas.get(start_x, start_y) {
        Some(cell) => cell,
        None => return vec![],
    };

    let new = Cell { ch, fg, bg };
    if target == new {
        return vec![]; // No-op: already the target color
    }

    let w = canvas.width;
    let h = canvas.height;
    let mut mutations = Vec::new();
    let mut visited = vec![false; w * h];
    let mut stack = vec![(start_x, start_y)];

    while let Some((x, y)) = stack.pop() {
        if x >= w || y >= h || visited[y * w + x] {
            continue;
        }
        if let Some(cell) = canvas.get(x, y) {
            if cell != target {
                continue;
            }
        } else {
            continue;
        }

        visited[y * w + x] = true;
        mutations.push(CellMutation {
            x,
            y,
            old: target,
            new,
        });

        if x > 0 {
            stack.push((x - 1, y));
        }
        if x + 1 < w {
            stack.push((x + 1, y));
        }
        if y > 0 {
            stack.push((x, y - 1));
        }
        if y + 1 < h {
            stack.push((x, y + 1));
        }
    }

    mutations
}

/// Compute the points of an ellipse outline inscribed in the bounding box
/// (x0,y0)-(x1,y1). Parametric sampling at canvas scale (max 128 cells)
/// produces a gap-free outline; duplicates are removed.
pub fn ellipse_points(x0: usize, y0: usize, x1: usize, y1: usize) -> Vec<(usize, usize)> {
    let min_x = x0.min(x1);
    let max_x = x0.max(x1);
    let min_y = y0.min(y1);
    let max_y = y0.max(y1);

    if min_x == max_x || min_y == max_y {
        // Degenerate: a line
        return bresenham_line(min_x, min_y, max_x, max_y);
    }

    let cx = (min_x + max_x) as f64 / 2.0;
    let cy = (min_y + max_y) as f64 / 2.0;
    let a = (max_x - min_x) as f64 / 2.0;
    let b = (max_y - min_y) as f64 / 2.0;

    let steps = (((a + b) * 8.0) as usize).max(16);
    let mut points = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for i in 0..steps {
        let theta = std::f64::consts::TAU * (i as f64) / (steps as f64);
        let px = (cx + a * theta.cos()).round() as i64;
        let py = (cy + b * theta.sin()).round() as i64;
        if px >= 0 && py >= 0 && seen.insert((px, py)) {
            points.push((px as usize, py as usize));
        }
    }
    points
}

/// Draw an ellipse inscribed in the bounding box (x0,y0)-(x1,y1).
#[allow(clippy::too_many_arguments)]
pub fn ellipse(
    canvas: &Canvas,
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
    ch: char,
    fg: Option<Rgb>,
    bg: Option<Rgb>,
    filled: bool,
) -> Vec<CellMutation> {
    let new = Cell { ch, fg, bg };
    let mut mutations = Vec::new();

    if filled {
        // Fill by inside test: |(x-cx)/a|^2 + |(y-cy)/b|^2 <= 1
        let min_x = x0.min(x1) as f64;
        let max_x = x0.max(x1) as f64;
        let min_y = y0.min(y1) as f64;
        let max_y = y0.max(y1) as f64;
        let cx = (min_x + max_x) / 2.0;
        let cy = (min_y + max_y) / 2.0;
        let a = ((max_x - min_x) / 2.0).max(0.5);
        let b = ((max_y - min_y) / 2.0).max(0.5);

        for y in (min_y as usize)..=(max_y as usize) {
            for x in (min_x as usize)..=(max_x as usize) {
                let dx = (x as f64 - cx) / a;
                let dy = (y as f64 - cy) / b;
                if dx * dx + dy * dy <= 1.0 + f64::EPSILON {
                    if let Some(old) = canvas.get(x, y) {
                        if old != new {
                            mutations.push(CellMutation { x, y, old, new });
                        }
                    }
                }
            }
        }
    } else {
        for (x, y) in ellipse_points(x0, y0, x1, y1) {
            if let Some(old) = canvas.get(x, y) {
                if old != new {
                    mutations.push(CellMutation { x, y, old, new });
                }
            }
        }
    }
    mutations
}

/// Linear interpolation between two colors.
fn lerp_rgb(from: Rgb, to: Rgb, t: f64) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    Rgb {
        r: (from.r as f64 + (to.r as f64 - from.r as f64) * t).round() as u8,
        g: (from.g as f64 + (to.g as f64 - from.g as f64) * t).round() as u8,
        b: (from.b as f64 + (to.b as f64 - from.b as f64) * t).round() as u8,
    }
}

/// Fill a region with a linear gradient between two colors.
///
/// `horizontal`: gradient runs left→right; otherwise top→bottom.
/// `dither`: instead of smooth per-cell RGB interpolation, use the classic
/// ANSI look — shade characters (`░▒▓`) blending `from` (bg) into `to` (fg)
/// across four bands per color step.
#[allow(clippy::too_many_arguments)]
pub fn gradient_fill(
    canvas: &Canvas,
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
    from: Rgb,
    to: Rgb,
    horizontal: bool,
    dither: bool,
) -> Vec<CellMutation> {
    use crate::cell::blocks;

    let min_x = x0.min(x1);
    let max_x = x0.max(x1).min(canvas.width.saturating_sub(1));
    let min_y = y0.min(y1);
    let max_y = y0.max(y1).min(canvas.height.saturating_sub(1));
    if min_x > max_x || min_y > max_y {
        return vec![];
    }

    let span = if horizontal {
        (max_x - min_x).max(1) as f64
    } else {
        (max_y - min_y).max(1) as f64
    };

    let mut mutations = Vec::new();
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let t = if horizontal {
                (x - min_x) as f64 / span
            } else {
                (y - min_y) as f64 / span
            };

            let new = if dither {
                // 4 sub-bands per gradient position: from-solid, light,
                // medium, dark shade of `to` over `from`.
                let band = (t * 4.0).min(3.999) as usize;
                match band {
                    0 => Cell { ch: blocks::FULL, fg: Some(from), bg: None },
                    1 => Cell { ch: blocks::SHADE_LIGHT, fg: Some(to), bg: Some(from) },
                    2 => Cell { ch: blocks::SHADE_MEDIUM, fg: Some(to), bg: Some(from) },
                    _ => Cell { ch: blocks::SHADE_DARK, fg: Some(to), bg: Some(from) },
                }
            } else {
                Cell { ch: blocks::FULL, fg: Some(lerp_rgb(from, to, t)), bg: None }
            };

            if let Some(old) = canvas.get(x, y) {
                if old != new {
                    mutations.push(CellMutation { x, y, old, new });
                }
            }
        }
    }
    mutations
}

/// Pick color from a canvas cell.
pub fn eyedropper(canvas: &Canvas, x: usize, y: usize) -> Option<(Option<Rgb>, Option<Rgb>, char)> {
    canvas.get(x, y).map(|cell| (cell.fg, cell.bg, cell.ch))
}

/// Compose a new cell from a drawing operation. All block types replace the
/// cell entirely — half-blocks stamp cleanly with the uncovered half transparent.
pub fn compose_cell(_existing: Cell, new_ch: char, new_fg: Option<Rgb>, new_bg: Option<Rgb>) -> Cell {
    Cell { ch: new_ch, fg: new_fg, bg: new_bg }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::blocks;

    const RED: Option<Rgb> = Some(Rgb { r: 205, g: 0, b: 0 });
    const BLUE: Option<Rgb> = Some(Rgb { r: 0, g: 0, b: 238 });
    const GREEN: Option<Rgb> = Some(Rgb { r: 0, g: 205, b: 0 });

    fn empty_cell() -> Cell {
        Cell::default()
    }

    #[test]
    fn test_bresenham_horizontal() {
        let points = bresenham_line(0, 0, 5, 0);
        assert_eq!(points.len(), 6);
        for (i, &(x, y)) in points.iter().enumerate() {
            assert_eq!(x, i);
            assert_eq!(y, 0);
        }
    }

    #[test]
    fn test_bresenham_vertical() {
        let points = bresenham_line(0, 0, 0, 5);
        assert_eq!(points.len(), 6);
        for (i, &(x, y)) in points.iter().enumerate() {
            assert_eq!(x, 0);
            assert_eq!(y, i);
        }
    }

    #[test]
    fn test_bresenham_diagonal() {
        let points = bresenham_line(0, 0, 5, 5);
        assert_eq!(points.len(), 6);
        for (i, &(x, y)) in points.iter().enumerate() {
            assert_eq!(x, i);
            assert_eq!(y, i);
        }
    }

    #[test]
    fn test_bresenham_single_point() {
        let points = bresenham_line(3, 3, 3, 3);
        assert_eq!(points.len(), 1);
        assert_eq!(points[0], (3, 3));
    }

    #[test]
    fn test_bresenham_steep() {
        let points = bresenham_line(0, 0, 2, 6);
        assert_eq!(points.first(), Some(&(0, 0)));
        assert_eq!(points.last(), Some(&(2, 6)));
        assert_eq!(points.len(), 7);
        for w in points.windows(2) {
            assert!(w[1].1 >= w[0].1);
        }
    }

    #[test]
    fn test_bresenham_shallow() {
        let points = bresenham_line(0, 0, 6, 2);
        assert_eq!(points.first(), Some(&(0, 0)));
        assert_eq!(points.last(), Some(&(6, 2)));
        assert_eq!(points.len(), 7);
        for w in points.windows(2) {
            assert!(w[1].0 >= w[0].0);
        }
    }

    #[test]
    fn test_bresenham_reverse() {
        let fwd = bresenham_line(0, 0, 5, 3);
        let rev = bresenham_line(5, 3, 0, 0);
        assert_eq!(fwd.len(), rev.len());
        for p in &fwd {
            assert!(rev.contains(p));
        }
    }

    #[test]
    fn test_rectangle_single_cell() {
        let canvas = Canvas::new();
        let mutations = rectangle(
            &canvas, 5, 5, 5, 5,
            blocks::FULL, RED, None, false,
        );
        assert_eq!(mutations.len(), 1);
        assert_eq!(mutations[0].x, 5);
        assert_eq!(mutations[0].y, 5);
    }

    #[test]
    fn test_rectangle_wide() {
        let canvas = Canvas::new();
        let mutations = rectangle(
            &canvas, 0, 0, 9, 0,
            blocks::FULL, RED, None, false,
        );
        assert_eq!(mutations.len(), 10);
    }

    #[test]
    fn test_rectangle_tall() {
        let canvas = Canvas::new();
        let mutations = rectangle(
            &canvas, 0, 0, 0, 7,
            blocks::FULL, RED, None, false,
        );
        assert_eq!(mutations.len(), 8);
    }

    #[test]
    fn test_flood_fill_boundary() {
        let mut canvas = Canvas::new();
        let wall = Cell {
            ch: blocks::FULL,
            fg: RED,
            bg: None,
        };
        for x in 0..3 {
            canvas.set(x, 0, wall);
            canvas.set(x, 2, wall);
        }
        canvas.set(0, 1, wall);
        canvas.set(2, 1, wall);
        let mutations = flood_fill(&canvas, 1, 1, blocks::FULL, BLUE, None);
        assert_eq!(mutations.len(), 1);
        assert_eq!(mutations[0].x, 1);
        assert_eq!(mutations[0].y, 1);
    }

    #[test]
    fn test_flood_fill_noop() {
        let canvas = Canvas::new();
        let mutations = flood_fill(
            &canvas,
            0,
            0,
            ' ',
            Some(Rgb::WHITE),
            None,
        );
        assert_eq!(mutations.len(), 0);
    }

    #[test]
    fn test_flood_fill_entire_canvas() {
        let canvas = Canvas::new();
        let mutations = flood_fill(
            &canvas,
            0,
            0,
            blocks::FULL,
            RED,
            None,
        );
        assert_eq!(mutations.len(), canvas.width * canvas.height);
    }

    #[test]
    fn test_rectangle_outline() {
        let canvas = Canvas::new();
        let mutations = rectangle(
            &canvas,
            0,
            0,
            3,
            3,
            blocks::FULL,
            RED,
            None,
            false,
        );
        assert_eq!(mutations.len(), 12);
    }

    #[test]
    fn test_rectangle_filled() {
        let canvas = Canvas::new();
        let mutations = rectangle(
            &canvas,
            0,
            0,
            3,
            3,
            blocks::FULL,
            RED,
            None,
            true,
        );
        assert_eq!(mutations.len(), 16);
    }

    // --- compose_cell tests ---

    #[test]
    fn compose_full_block_replaces_entirely() {
        let existing = Cell { ch: blocks::UPPER_HALF, fg: RED, bg: BLUE };
        let result = compose_cell(existing, blocks::FULL, GREEN, None);
        assert_eq!(result, Cell { ch: blocks::FULL, fg: GREEN, bg: None });
    }

    #[test]
    fn compose_empty_block_replaces_entirely() {
        let existing = Cell { ch: blocks::FULL, fg: RED, bg: None };
        let result = compose_cell(existing, ' ', Some(Rgb::WHITE), None);
        assert_eq!(result, Cell { ch: ' ', fg: Some(Rgb::WHITE), bg: None });
    }

    #[test]
    fn compose_upper_half_on_empty() {
        let result = compose_cell(empty_cell(), blocks::UPPER_HALF, RED, None);
        assert_eq!(result.ch, blocks::UPPER_HALF);
        assert_eq!(result.fg, RED);
        assert_eq!(result.bg, None);
    }

    #[test]
    fn compose_lower_half_on_empty() {
        let result = compose_cell(empty_cell(), blocks::LOWER_HALF, RED, None);
        assert_eq!(result.ch, blocks::LOWER_HALF);
        assert_eq!(result.fg, RED);
        assert_eq!(result.bg, None);
    }

    #[test]
    fn compose_upper_red_then_lower_blue() {
        let existing = Cell { ch: blocks::UPPER_HALF, fg: RED, bg: None };
        let result = compose_cell(existing, blocks::LOWER_HALF, BLUE, None);
        assert_eq!(result.ch, blocks::LOWER_HALF);
        assert_eq!(result.fg, BLUE);
        assert_eq!(result.bg, None);
    }

    #[test]
    fn compose_lower_blue_then_upper_red() {
        let existing = Cell { ch: blocks::LOWER_HALF, fg: BLUE, bg: None };
        let result = compose_cell(existing, blocks::UPPER_HALF, RED, None);
        assert_eq!(result.ch, blocks::UPPER_HALF);
        assert_eq!(result.fg, RED);
        assert_eq!(result.bg, None);
    }

    #[test]
    fn compose_lower_half_replaces_regardless_of_existing() {
        let existing = Cell { ch: blocks::UPPER_HALF, fg: RED, bg: None };
        let result = compose_cell(existing, blocks::LOWER_HALF, RED, None);
        assert_eq!(result.ch, blocks::LOWER_HALF);
        assert_eq!(result.fg, RED);
        assert_eq!(result.bg, None);
    }

    #[test]
    fn compose_left_half_on_empty() {
        let result = compose_cell(empty_cell(), blocks::LEFT_HALF, RED, None);
        assert_eq!(result.ch, blocks::LEFT_HALF);
        assert_eq!(result.fg, RED);
        assert_eq!(result.bg, None);
    }

    #[test]
    fn compose_right_half_on_empty() {
        let result = compose_cell(empty_cell(), blocks::RIGHT_HALF, RED, None);
        assert_eq!(result.ch, blocks::RIGHT_HALF);
        assert_eq!(result.fg, RED);
        assert_eq!(result.bg, None);
    }

    #[test]
    fn compose_left_then_right_horizontal() {
        let existing = Cell { ch: blocks::LEFT_HALF, fg: RED, bg: None };
        let result = compose_cell(existing, blocks::RIGHT_HALF, BLUE, None);
        assert_eq!(result.ch, blocks::RIGHT_HALF);
        assert_eq!(result.fg, BLUE);
        assert_eq!(result.bg, None);
    }

    #[test]
    fn compose_right_half_replaces_regardless_of_existing() {
        let existing = Cell { ch: blocks::LEFT_HALF, fg: RED, bg: None };
        let result = compose_cell(existing, blocks::RIGHT_HALF, RED, None);
        assert_eq!(result.ch, blocks::RIGHT_HALF);
        assert_eq!(result.fg, RED);
        assert_eq!(result.bg, None);
    }

    #[test]
    fn compose_cross_axis_replaces_entirely() {
        let existing = Cell { ch: blocks::LEFT_HALF, fg: RED, bg: None };
        let result = compose_cell(existing, blocks::UPPER_HALF, BLUE, None);
        assert_eq!(result.ch, blocks::UPPER_HALF);
        assert_eq!(result.fg, BLUE);
        assert_eq!(result.bg, None);
    }

    #[test]
    fn compose_half_on_full_replaces_entirely() {
        let existing = Cell { ch: blocks::FULL, fg: RED, bg: None };
        let result = compose_cell(existing, blocks::UPPER_HALF, BLUE, None);
        assert_eq!(result.ch, blocks::UPPER_HALF);
        assert_eq!(result.fg, BLUE);
        assert_eq!(result.bg, None);
    }

    #[test]
    fn compose_idempotent_same_half_same_color() {
        let existing = Cell { ch: blocks::UPPER_HALF, fg: RED, bg: None };
        let result = compose_cell(existing, blocks::UPPER_HALF, RED, None);
        assert_eq!(result, existing);
    }

    // --- Cycle 15 QA: Shade character tool tests ---

    #[test]
    fn test_pencil_shade_char() {
        let mut canvas = Canvas::new();
        let mutations = pencil(&canvas, 3, 5, blocks::SHADE_LIGHT, RED, None);
        assert_eq!(mutations.len(), 1);
        assert_eq!(mutations[0].new.ch, blocks::SHADE_LIGHT);
        assert_eq!(mutations[0].new.fg, RED);
        for m in &mutations {
            canvas.set(m.x, m.y, m.new);
        }
        let cell = canvas.get(3, 5).unwrap();
        assert_eq!(cell.ch, blocks::SHADE_LIGHT);
    }

    #[test]
    fn test_fill_shade_char() {
        let canvas = Canvas::new();
        // Fill entire empty region with shade char
        let mutations = flood_fill(&canvas, 0, 0, blocks::SHADE_MEDIUM, RED, None);
        assert!(!mutations.is_empty(), "Fill should produce mutations");
        // All mutations should use shade char
        for m in &mutations {
            assert_eq!(m.new.ch, blocks::SHADE_MEDIUM);
            assert_eq!(m.new.fg, RED);
        }
    }

    #[test]
    fn test_eraser_shade_cell() {
        let mut canvas = Canvas::new();
        // Place a shade char
        canvas.set(2, 3, Cell { ch: blocks::SHADE_DARK, fg: RED, bg: None });
        // Erase it
        let mutations = eraser(&canvas, 2, 3);
        assert_eq!(mutations.len(), 1);
        assert_eq!(mutations[0].new.ch, ' ');
        assert_eq!(mutations[0].new.fg, Some(Rgb::WHITE));
        assert_eq!(mutations[0].new.bg, None);
    }

    #[test]
    fn test_ellipse_outline_touches_extremes() {
        let canvas = Canvas::new_with_size(20, 20);
        let mutations = ellipse(&canvas, 2, 2, 14, 10, blocks::FULL, RED, None, false);
        assert!(!mutations.is_empty());
        let points: Vec<(usize, usize)> = mutations.iter().map(|m| (m.x, m.y)).collect();
        // Midpoints of each bounding box edge must be on the outline
        assert!(points.contains(&(8, 2)), "top midpoint missing");
        assert!(points.contains(&(8, 10)), "bottom midpoint missing");
        assert!(points.contains(&(2, 6)), "left midpoint missing");
        assert!(points.contains(&(14, 6)), "right midpoint missing");
        // All points within bounding box
        assert!(points.iter().all(|&(x, y)| (2..=14).contains(&x) && (2..=10).contains(&y)));
        // No duplicates
        let unique: std::collections::HashSet<_> = points.iter().collect();
        assert_eq!(unique.len(), points.len());
    }

    #[test]
    fn test_ellipse_filled_contains_center() {
        let canvas = Canvas::new_with_size(20, 20);
        let mutations = ellipse(&canvas, 2, 2, 14, 10, blocks::FULL, RED, None, true);
        let points: Vec<(usize, usize)> = mutations.iter().map(|m| (m.x, m.y)).collect();
        assert!(points.contains(&(8, 6)), "center missing from filled ellipse");
        // Corners of the bounding box must NOT be inside the ellipse
        assert!(!points.contains(&(2, 2)));
        assert!(!points.contains(&(14, 10)));
        // Filled has more cells than the outline
        let outline = ellipse(&canvas, 2, 2, 14, 10, blocks::FULL, RED, None, false);
        assert!(points.len() > outline.len());
    }

    #[test]
    fn test_ellipse_degenerate_is_line() {
        let canvas = Canvas::new_with_size(20, 20);
        let mutations = ellipse(&canvas, 3, 5, 10, 5, blocks::FULL, RED, None, false);
        assert_eq!(mutations.len(), 8); // horizontal line 3..=10
        assert!(mutations.iter().all(|m| m.y == 5));
    }

    #[test]
    fn test_gradient_smooth_endpoints() {
        let canvas = Canvas::new_with_size(16, 16);
        let from = Rgb::new(0, 0, 0);
        let to = Rgb::new(255, 255, 255);
        let mutations = gradient_fill(&canvas, 0, 0, 15, 15, from, to, false, false);
        assert_eq!(mutations.len(), 256);
        // Vertical: top row is `from`, bottom row is `to`
        let top = mutations.iter().find(|m| m.x == 0 && m.y == 0).unwrap();
        let bottom = mutations.iter().find(|m| m.x == 0 && m.y == 15).unwrap();
        assert_eq!(top.new.fg, Some(from));
        assert_eq!(bottom.new.fg, Some(to));
        // Middle is in between
        let mid = mutations.iter().find(|m| m.x == 0 && m.y == 8).unwrap();
        let mid_fg = mid.new.fg.unwrap();
        assert!(mid_fg.r > 0 && mid_fg.r < 255);
    }

    #[test]
    fn test_gradient_horizontal_direction() {
        let canvas = Canvas::new_with_size(16, 16);
        let from = Rgb::new(255, 0, 0);
        let to = Rgb::new(0, 0, 255);
        let mutations = gradient_fill(&canvas, 0, 0, 15, 15, from, to, true, false);
        let left = mutations.iter().find(|m| m.x == 0 && m.y == 5).unwrap();
        let right = mutations.iter().find(|m| m.x == 15 && m.y == 5).unwrap();
        assert_eq!(left.new.fg, Some(from));
        assert_eq!(right.new.fg, Some(to));
    }

    #[test]
    fn test_gradient_dither_uses_shades() {
        let canvas = Canvas::new_with_size(16, 16);
        let from = Rgb::new(0, 0, 128);
        let to = Rgb::new(255, 136, 0);
        let mutations = gradient_fill(&canvas, 0, 0, 15, 15, from, to, false, true);
        assert_eq!(mutations.len(), 256);
        let chars: std::collections::HashSet<char> =
            mutations.iter().map(|m| m.new.ch).collect();
        // All four band characters appear across a full-height gradient
        assert!(chars.contains(&blocks::FULL));
        assert!(chars.contains(&blocks::SHADE_LIGHT));
        assert!(chars.contains(&blocks::SHADE_MEDIUM));
        assert!(chars.contains(&blocks::SHADE_DARK));
    }

    #[test]
    fn test_gradient_clamps_to_canvas() {
        let canvas = Canvas::new_with_size(8, 8);
        let mutations = gradient_fill(
            &canvas, 0, 0, 100, 100,
            Rgb::new(0, 0, 0), Rgb::new(255, 255, 255),
            false, false,
        );
        assert_eq!(mutations.len(), 64); // clamped to 8x8
        assert!(mutations.iter().all(|m| m.x < 8 && m.y < 8));
    }
}
