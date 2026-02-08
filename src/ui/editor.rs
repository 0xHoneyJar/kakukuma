use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

use crate::app::App;
use crate::canvas::{CANVAS_HEIGHT, CANVAS_WIDTH};
use crate::cell::BlockChar;
use crate::input::CanvasArea;
use crate::tools::{self, ToolState};

/// Render the canvas editor and return the screen area for mouse mapping.
pub fn render(f: &mut Frame, app: &App, area: Rect) -> CanvasArea {
    // Canvas needs 64 cols (32 cells * 2 chars) and 32 rows
    let canvas_w = (CANVAS_WIDTH * 2) as u16;
    let canvas_h = CANVAS_HEIGHT as u16;

    // Center the canvas in the available area
    let offset_x = (area.width.saturating_sub(canvas_w)) / 2;
    let offset_y = (area.height.saturating_sub(canvas_h)) / 2;

    let canvas_rect = Rect::new(
        area.x + offset_x,
        area.y + offset_y,
        canvas_w.min(area.width),
        canvas_h.min(area.height),
    );

    let widget = CanvasWidget {
        app,
        show_grid: app.show_grid,
    };
    f.render_widget(widget, canvas_rect);

    CanvasArea {
        left: canvas_rect.x,
        top: canvas_rect.y,
        width: canvas_rect.width,
        height: canvas_rect.height,
    }
}

/// Render the canvas at 1:1 as a preview (no grid, no cursor).
pub fn render_preview(f: &mut Frame, app: &App, area: Rect) {
    let canvas_w = (CANVAS_WIDTH * 2) as u16;
    let canvas_h = CANVAS_HEIGHT as u16;

    let offset_x = (area.width.saturating_sub(canvas_w)) / 2;
    let offset_y = (area.height.saturating_sub(canvas_h)) / 2;

    let canvas_rect = Rect::new(
        area.x + offset_x,
        area.y + offset_y,
        canvas_w.min(area.width),
        canvas_h.min(area.height),
    );

    let widget = PreviewWidget { app };
    f.render_widget(widget, canvas_rect);
}

struct CanvasWidget<'a> {
    app: &'a App,
    show_grid: bool,
}

impl<'a> CanvasWidget<'a> {
    fn is_in_tool_preview(&self, x: usize, y: usize) -> bool {
        let cursor = match self.app.cursor {
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
        for y in 0..CANVAS_HEIGHT {
            for x in 0..CANVAS_WIDTH {
                let screen_x = area.x + (x as u16) * 2;
                let screen_y = area.y + y as u16;

                if screen_x + 1 >= area.x + area.width || screen_y >= area.y + area.height {
                    continue;
                }

                if let Some(cell) = self.app.canvas.get(x, y) {
                    let is_cursor = self.app.cursor == Some((x, y));
                    let mut ch = cell.block.to_char();
                    let mut fg = cell.fg.to_ratatui();
                    let mut bg = cell.bg.to_ratatui();

                    // Full block: set bg=fg so glyph gaps are invisible
                    if cell.block == BlockChar::Full {
                        bg = fg;
                    }

                    // Grid: checkerboard on empty cells
                    if self.show_grid && cell.block == BlockChar::Empty {
                        if (x + y) % 2 == 0 {
                            bg = Color::Indexed(236); // Very dark gray
                        } else {
                            bg = Color::Indexed(234); // Slightly different dark
                        }
                    }

                    // Cursor highlight
                    if is_cursor {
                        // Invert colors for cursor
                        std::mem::swap(&mut fg, &mut bg);
                    }

                    // Symmetry axis indicator
                    let on_h_axis = self.app.symmetry.has_horizontal()
                        && (x == CANVAS_WIDTH / 2 - 1 || x == CANVAS_WIDTH / 2);
                    let on_v_axis = self.app.symmetry.has_vertical()
                        && (y == CANVAS_HEIGHT / 2 - 1 || y == CANVAS_HEIGHT / 2);

                    if (on_h_axis || on_v_axis) && cell.block == BlockChar::Empty && !is_cursor {
                        bg = Color::Indexed(238); // Slightly lighter to show axis
                    }

                    // Tool preview overlay (line/rect in progress)
                    let in_preview = self.is_in_tool_preview(x, y);
                    if in_preview && !is_cursor {
                        let c = self.app.color.to_ratatui();
                        fg = c;
                        bg = c;
                        ch = BlockChar::Full.to_char();
                    }

                    let style = Style::default().fg(fg).bg(bg);
                    let ch_str: String = std::iter::repeat_n(ch, 2).collect();
                    buf.set_string(screen_x, screen_y, &ch_str, style);
                }
            }
        }
    }
}

struct PreviewWidget<'a> {
    app: &'a App,
}

impl<'a> Widget for PreviewWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        for y in 0..CANVAS_HEIGHT {
            for x in 0..CANVAS_WIDTH {
                let screen_x = area.x + (x as u16) * 2;
                let screen_y = area.y + y as u16;

                if screen_x + 1 >= area.x + area.width || screen_y >= area.y + area.height {
                    continue;
                }

                if let Some(cell) = self.app.canvas.get(x, y) {
                    let ch = cell.block.to_char();
                    let fg = cell.fg.to_ratatui();
                    let mut bg = cell.bg.to_ratatui();
                    // Full block: set bg=fg so glyph gaps are invisible
                    if cell.block == BlockChar::Full {
                        bg = fg;
                    }
                    let style = Style::default().fg(fg).bg(bg);
                    let ch_str: String = std::iter::repeat_n(ch, 2).collect();
                    buf.set_string(screen_x, screen_y, &ch_str, style);
                }
            }
        }
    }
}
