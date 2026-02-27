use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, BorderType, Widget};

use crate::app::App;
use crate::cell::{blocks, is_half_block, Cell, resolve_half_block};
use crate::input::CanvasArea;
use crate::theme::Theme;
use crate::tools::{self, ToolState};

/// Direction of a symmetry axis at a given cell position.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum AxisDirection {
    Vertical,    // │ — vertical line (horizontal symmetry mirrors left/right)
    Horizontal,  // ─ — horizontal line (vertical symmetry mirrors top/bottom)
    Intersection, // ┼ — quad symmetry intersection
}

/// Return axis glyph, fg, and bg for a cell on a symmetry axis.
/// - Empty cells: axis glyph with `theme.accent` fg and `theme.panel_bg` bg
/// - Occupied cells: keep glyph, tint fg to `theme.dim`
fn render_axis_cell(
    ch: char,
    _fg: Color,
    bg: Color,
    direction: AxisDirection,
    theme: &Theme,
    is_empty: bool,
) -> (char, Color, Color) {
    let glyph = match direction {
        AxisDirection::Vertical => '\u{2502}',     // │
        AxisDirection::Horizontal => '\u{2500}',   // ─
        AxisDirection::Intersection => '\u{253C}',  // ┼
    };

    if is_empty {
        // Empty cell: show axis glyph with accent color
        (glyph, theme.accent, theme.panel_bg)
    } else if ch == ' ' {
        // Background-only cell: show axis glyph, preserve existing bg
        (glyph, theme.accent, bg)
    } else {
        // Occupied cell: keep glyph, tint fg
        (ch, theme.dim, bg)
    }
}

/// Return the visual background color for an empty/transparent cell position.
fn grid_bg(x: usize, y: usize, show_grid: bool, theme: &Theme) -> Color {
    if show_grid {
        if (x + y).is_multiple_of(2) {
            theme.grid_even
        } else {
            theme.grid_odd
        }
    } else {
        Color::Reset
    }
}

/// Thin wrapper around `cell::resolve_half_block` that maps transparent halves
/// to grid background colors for terminal display.
fn resolve_half_block_for_display(cell: Cell, x: usize, y: usize, show_grid: bool, theme: &Theme) -> (char, Color, Color) {
    let resolved = resolve_half_block(&cell).unwrap();

    if resolved.ch == ' ' {
        return (' ', Color::Reset, grid_bg(x, y, show_grid, theme));
    }

    let fg = resolved.fg.map_or(Color::Reset, |rgb| rgb.to_ratatui());
    let bg = resolved.bg.map_or(grid_bg(x, y, show_grid, theme), |rgb| rgb.to_ratatui());
    (resolved.ch, fg, bg)
}

/// Render the canvas editor and return the screen area for mouse mapping.
pub fn render(f: &mut Frame, app: &App, area: Rect) -> CanvasArea {
    let theme = app.theme();
    let zoom = app.zoom as u16;

    // Viewport: how many canvas cells fit in the available area
    let inner_w = area.width.saturating_sub(2); // border
    let inner_h = area.height.saturating_sub(2);
    let vp_w = (inner_w / zoom) as usize;
    let vp_h = match zoom {
        4 => (inner_h / 2) as usize,
        _ => inner_h as usize,
    };

    // Visible canvas dimensions (clamped to actual canvas size)
    let vis_w = vp_w.min(app.canvas.width.saturating_sub(app.viewport_x));
    let vis_h = vp_h.min(app.canvas.height.saturating_sub(app.viewport_y));

    let canvas_w = vis_w as u16 * zoom;
    let canvas_h = match zoom {
        4 => vis_h as u16 * 2,
        _ => vis_h as u16,
    };

    // Add 2 for border on each axis
    let bordered_w = canvas_w + 2;
    let bordered_h = canvas_h + 2;

    // Center the bordered area
    let offset_x = (area.width.saturating_sub(bordered_w)) / 2;
    let offset_y = (area.height.saturating_sub(bordered_h)) / 2;

    let bordered_rect = Rect::new(
        area.x + offset_x,
        area.y + offset_y,
        bordered_w.min(area.width),
        bordered_h.min(area.height),
    );

    // Render the border
    let border = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.separator));
    let inner_rect = border.inner(bordered_rect);
    f.render_widget(border, bordered_rect);

    // Scroll indicators on border edges
    let buf = f.buffer_mut();
    let ind_style = Style::default().fg(theme.dim);
    if app.viewport_x > 0 {
        // Left arrow on left border
        let mid_y = bordered_rect.y + bordered_rect.height / 2;
        if mid_y < bordered_rect.y + bordered_rect.height {
            buf.set_string(bordered_rect.x, mid_y, "\u{25C0}", ind_style);
        }
    }
    if app.viewport_x + vis_w < app.canvas.width {
        // Right arrow on right border
        let mid_y = bordered_rect.y + bordered_rect.height / 2;
        let right_x = bordered_rect.x + bordered_rect.width.saturating_sub(1);
        if mid_y < bordered_rect.y + bordered_rect.height {
            buf.set_string(right_x, mid_y, "\u{25B6}", ind_style);
        }
    }
    if app.viewport_y > 0 {
        // Up arrow on top border
        let mid_x = bordered_rect.x + bordered_rect.width / 2;
        if mid_x < bordered_rect.x + bordered_rect.width {
            buf.set_string(mid_x, bordered_rect.y, "\u{25B2}", ind_style);
        }
    }
    if app.viewport_y + vis_h < app.canvas.height {
        // Down arrow on bottom border
        let mid_x = bordered_rect.x + bordered_rect.width / 2;
        let bot_y = bordered_rect.y + bordered_rect.height.saturating_sub(1);
        if mid_x < bordered_rect.x + bordered_rect.width {
            buf.set_string(mid_x, bot_y, "\u{25BC}", ind_style);
        }
    }

    // Render canvas inside the border
    let widget = CanvasWidget { app };
    f.render_widget(widget, inner_rect);

    CanvasArea {
        left: inner_rect.x,
        top: inner_rect.y,
        width: inner_rect.width,
        height: inner_rect.height,
        viewport_w: vp_w,
        viewport_h: vp_h,
    }
}

struct CanvasWidget<'a> {
    app: &'a App,
}

impl<'a> CanvasWidget<'a> {
    fn is_in_tool_preview(&self, x: usize, y: usize) -> bool {
        let cursor = match self.app.effective_cursor() {
            Some(c) => c,
            None => return false,
        };
        match &self.app.tool_state {
            ToolState::LineStart { x: x0, y: y0 } => {
                let points = tools::bresenham_line(*x0, *y0, cursor.0, cursor.1);
                points.contains(&(x, y))
            }
            ToolState::RectStart { x: x0, y: y0 } => {
                let min_x = (*x0).min(cursor.0);
                let max_x = (*x0).max(cursor.0);
                let min_y = (*y0).min(cursor.1);
                let max_y = (*y0).max(cursor.1);
                let is_border = x == min_x || x == max_x || y == min_y || y == max_y;
                x >= min_x && x <= max_x && y >= min_y && y <= max_y && is_border
            }
            ToolState::Idle => false,
        }
    }
}

impl<'a> Widget for CanvasWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let zoom = self.app.zoom;
        let show_grid = zoom > 1;
        let theme = self.app.theme();
        let vp_x = self.app.viewport_x;
        let vp_y = self.app.viewport_y;

        // Viewport dimensions in canvas cells
        let vp_w = (area.width / zoom as u16) as usize;
        let vp_h = match zoom {
            4 => (area.height / 2) as usize,
            _ => area.height as usize,
        };

        let vis_w = vp_w.min(self.app.canvas.width.saturating_sub(vp_x));
        let vis_h = vp_h.min(self.app.canvas.height.saturating_sub(vp_y));

        for vy in 0..vis_h {
            for vx in 0..vis_w {
                let x = vx + vp_x;
                let y = vy + vp_y;
                let screen_x = area.x + (vx as u16) * zoom as u16;
                let screen_y = match zoom {
                    4 => area.y + (vy as u16) * 2,
                    _ => area.y + vy as u16,
                };

                // Bounds check
                if screen_x >= area.x + area.width || screen_y >= area.y + area.height {
                    continue;
                }

                let cell = match self.app.canvas.get(x, y) {
                    Some(c) => c,
                    None => continue,
                };

                let is_cursor = self.app.effective_cursor() == Some((x, y));

                // Tool preview overlay (line/rect in progress)
                let render_cell = if self.is_in_tool_preview(x, y) && !is_cursor {
                    tools::compose_cell(
                        cell,
                        self.app.active_block,
                        Some(self.app.color),
                        None,
                    )
                } else {
                    cell
                };

                // Resolve to (char, fg, bg) using unified path
                let (mut ch_out, mut fg, mut bg) = if render_cell.ch == blocks::FULL {
                    let c = render_cell.fg.map_or(Color::Reset, |rgb| rgb.to_ratatui());
                    ('\u{2588}', c, c)
                } else if render_cell.is_empty() {
                    (' ', Color::Reset, grid_bg(x, y, show_grid, theme))
                } else if is_half_block(render_cell.ch) {
                    resolve_half_block_for_display(render_cell, x, y, show_grid, theme)
                } else {
                    // Fractional fills, shades, and other single-color blocks
                    let fg_color = render_cell.fg.map_or(Color::Reset, |rgb| rgb.to_ratatui());
                    (render_cell.ch, fg_color, grid_bg(x, y, show_grid, theme))
                };

                // Symmetry axis visualization
                let canvas_w = self.app.canvas.width;
                let canvas_h = self.app.canvas.height;
                let mid_x = canvas_w / 2;
                let mid_y = canvas_h / 2;
                let on_v_line = self.app.symmetry.has_horizontal()
                    && (x == mid_x.saturating_sub(1) || x == mid_x);
                let on_h_line = self.app.symmetry.has_vertical()
                    && (y == mid_y.saturating_sub(1) || y == mid_y);
                if (on_v_line || on_h_line) && !is_cursor {
                    let direction = match (on_v_line, on_h_line) {
                        (true, true) => AxisDirection::Intersection,
                        (true, false) => AxisDirection::Vertical,
                        (false, true) => AxisDirection::Horizontal,
                        _ => unreachable!(),
                    };
                    let result = render_axis_cell(
                        ch_out, fg, bg, direction, theme, render_cell.is_empty(),
                    );
                    ch_out = result.0;
                    fg = result.1;
                    bg = result.2;
                }

                // Cursor inversion
                if is_cursor {
                    std::mem::swap(&mut fg, &mut bg);
                }

                let style = Style::default().fg(fg).bg(bg);

                // Paint across zoom width
                match zoom {
                    1 => {
                        buf.set_string(screen_x, screen_y, ch_out.to_string(), style);
                    }
                    2 => {
                        let s: String = std::iter::repeat_n(ch_out, 2).collect();
                        buf.set_string(screen_x, screen_y, &s, style);
                    }
                    4 => {
                        let s: String = std::iter::repeat_n(ch_out, 4).collect();
                        buf.set_string(screen_x, screen_y, &s, style);
                        // Second row: same content
                        if screen_y + 1 < area.y + area.height {
                            buf.set_string(screen_x, screen_y + 1, &s, style);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::Rgb;
    use crate::theme::WARM;

    // --- grid_bg tests ---

    #[test]
    fn grid_bg_even_cell_with_grid() {
        assert_eq!(grid_bg(0, 0, true, &WARM), WARM.grid_even);
        assert_eq!(grid_bg(2, 4, true, &WARM), WARM.grid_even);
    }

    #[test]
    fn grid_bg_odd_cell_with_grid() {
        assert_eq!(grid_bg(1, 0, true, &WARM), WARM.grid_odd);
        assert_eq!(grid_bg(0, 1, true, &WARM), WARM.grid_odd);
    }

    #[test]
    fn grid_bg_without_grid() {
        assert_eq!(grid_bg(0, 0, false, &WARM), Color::Reset);
        assert_eq!(grid_bg(1, 0, false, &WARM), Color::Reset);
    }

    // --- resolve_half_block_for_display tests ---

    const RED: Rgb = Rgb { r: 205, g: 0, b: 0 };
    const BLUE: Rgb = Rgb { r: 0, g: 0, b: 238 };

    fn make_cell(ch: char, fg: Option<Rgb>, bg: Option<Rgb>) -> Cell {
        Cell { ch, fg, bg }
    }

    #[test]
    fn upper_half_one_transparent_bottom() {
        let (ch, fg, bg) = resolve_half_block_for_display(make_cell(blocks::UPPER_HALF, Some(RED), None), 0, 0, true, &WARM);
        assert_eq!(ch, '▀');
        assert_eq!(fg, Color::Indexed(1));
        assert_eq!(bg, WARM.grid_even);
    }

    #[test]
    fn upper_half_both_opaque() {
        let (ch, fg, bg) = resolve_half_block_for_display(make_cell(blocks::UPPER_HALF, Some(RED), Some(BLUE)), 0, 0, true, &WARM);
        assert_eq!(ch, '▀');
        assert_eq!(fg, Color::Indexed(1));
        assert_eq!(bg, Color::Indexed(4));
    }

    #[test]
    fn upper_half_one_transparent_top_flips() {
        let (ch, fg, bg) = resolve_half_block_for_display(make_cell(blocks::UPPER_HALF, None, Some(BLUE)), 0, 0, true, &WARM);
        assert_eq!(ch, '▄');
        assert_eq!(fg, Color::Indexed(4));
        assert_eq!(bg, WARM.grid_even);
    }

    #[test]
    fn upper_half_both_transparent() {
        let (ch, _fg, bg) = resolve_half_block_for_display(make_cell(blocks::UPPER_HALF, None, None), 0, 0, true, &WARM);
        assert_eq!(ch, ' ');
        assert_eq!(bg, WARM.grid_even);
    }

    #[test]
    fn left_half_one_transparent_right() {
        let (ch, fg, bg) = resolve_half_block_for_display(make_cell(blocks::LEFT_HALF, Some(RED), None), 1, 0, true, &WARM);
        assert_eq!(ch, '▌');
        assert_eq!(fg, Color::Indexed(1));
        assert_eq!(bg, WARM.grid_odd);
    }

    #[test]
    fn left_half_flips_when_left_transparent() {
        let (ch, fg, bg) = resolve_half_block_for_display(make_cell(blocks::LEFT_HALF, None, Some(RED)), 0, 0, true, &WARM);
        assert_eq!(ch, '▐');
        assert_eq!(fg, Color::Indexed(1));
        assert_eq!(bg, WARM.grid_even);
    }

    #[test]
    fn lower_half_defensive() {
        let (ch, fg, bg) = resolve_half_block_for_display(make_cell(blocks::LOWER_HALF, Some(BLUE), None), 0, 0, true, &WARM);
        assert_eq!(ch, '▄');
        assert_eq!(fg, Color::Indexed(4));
        assert_eq!(bg, WARM.grid_even);
    }

    #[test]
    fn right_half_defensive() {
        let (ch, fg, bg) = resolve_half_block_for_display(make_cell(blocks::RIGHT_HALF, Some(RED), None), 0, 0, true, &WARM);
        assert_eq!(ch, '▐');
        assert_eq!(fg, Color::Indexed(1));
        assert_eq!(bg, WARM.grid_even);
    }

    #[test]
    fn resolve_grid_off_uses_reset() {
        let (ch, fg, bg) = resolve_half_block_for_display(make_cell(blocks::UPPER_HALF, Some(RED), None), 0, 0, false, &WARM);
        assert_eq!(ch, '▀');
        assert_eq!(fg, Color::Indexed(1));
        assert_eq!(bg, Color::Reset);
    }

    #[test]
    fn left_half_both_opaque() {
        let (ch, fg, bg) = resolve_half_block_for_display(make_cell(blocks::LEFT_HALF, Some(RED), Some(BLUE)), 0, 0, true, &WARM);
        assert_eq!(ch, '▌');
        assert_eq!(fg, Color::Indexed(1));
        assert_eq!(bg, Color::Indexed(4));
    }

    // --- render_axis_cell tests ---

    #[test]
    fn test_axis_empty_cell() {
        // Empty cell on vertical axis → │ glyph with accent fg, panel_bg bg
        let (ch, fg, bg) = render_axis_cell(' ', Color::Reset, Color::Reset, AxisDirection::Vertical, &WARM, true);
        assert_eq!(ch, '│');
        assert_eq!(fg, WARM.accent);
        assert_eq!(bg, WARM.panel_bg);
    }

    #[test]
    fn test_axis_occupied_cell() {
        // Occupied cell (has visible glyph) → keep glyph, tint fg to dim
        let (ch, fg, bg) = render_axis_cell('█', Color::Indexed(1), Color::Indexed(4), AxisDirection::Vertical, &WARM, false);
        assert_eq!(ch, '█', "Should preserve original glyph");
        assert_eq!(fg, WARM.dim, "Should tint fg to dim");
        assert_eq!(bg, Color::Indexed(4), "Should preserve bg");
    }

    #[test]
    fn test_axis_bg_only_cell() {
        // Space char with bg color → show axis glyph, preserve bg
        let (ch, fg, bg) = render_axis_cell(' ', Color::Reset, Color::Indexed(4), AxisDirection::Horizontal, &WARM, false);
        assert_eq!(ch, '─', "Should show horizontal axis glyph");
        assert_eq!(fg, WARM.accent, "Should use accent fg");
        assert_eq!(bg, Color::Indexed(4), "Should preserve existing bg");
    }

    #[test]
    fn test_axis_quad_intersection() {
        // Intersection of both axes → ┼ character
        let (ch, fg, bg) = render_axis_cell(' ', Color::Reset, Color::Reset, AxisDirection::Intersection, &WARM, true);
        assert_eq!(ch, '┼');
        assert_eq!(fg, WARM.accent);
        assert_eq!(bg, WARM.panel_bg);
    }
}
