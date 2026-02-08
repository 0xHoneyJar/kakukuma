use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;

// Warm accent colors (shared with mod.rs)
const WARM_GOLDEN: Color = Color::Indexed(220);
const WARM_GRAY_BG: Color = Color::Indexed(235);
const WARM_GRAY_DIM: Color = Color::Indexed(243);

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let mut spans = Vec::new();

    // Status message takes priority
    if let Some(ref msg) = app.status_message {
        spans.push(Span::styled(
            format!(" {} ", msg.text),
            Style::default().fg(WARM_GOLDEN).bg(WARM_GRAY_BG),
        ));
    } else {
        // Default shortcut hints — dim undo/redo when unavailable
        let undo_fg = if app.history.can_undo() { Color::White } else { WARM_GRAY_DIM };
        let undo_label_fg = if app.history.can_undo() { Color::Gray } else { WARM_GRAY_DIM };
        let redo_fg = if app.history.can_redo() { Color::White } else { WARM_GRAY_DIM };
        let redo_label_fg = if app.history.can_redo() { Color::Gray } else { WARM_GRAY_DIM };

        let hints: &[(&str, &str, Color, Color)] = &[
            ("^S", " Save ", Color::White, Color::Gray),
            ("^E", " Export ", Color::White, Color::Gray),
            ("^Z", " Undo ", undo_fg, undo_label_fg),
            ("^Y", " Redo ", redo_fg, redo_label_fg),
            ("?", " Help ", Color::White, Color::Gray),
            ("Q", " Quit ", Color::White, Color::Gray),
        ];
        for &(key, label, key_fg, label_fg) in hints {
            spans.push(Span::styled(
                key,
                Style::default().fg(key_fg).bg(WARM_GRAY_BG),
            ));
            spans.push(Span::styled(
                label,
                Style::default().fg(label_fg).bg(WARM_GRAY_BG),
            ));
        }
    }

    // Cursor position on the right side
    if let Some((x, y)) = app.cursor {
        let pos_text = format!(" ({},{}) ", x, y);
        let padding = (area.width as usize)
            .saturating_sub(spans.iter().map(|s| s.content.len()).sum::<usize>() + pos_text.len());
        spans.push(Span::styled(
            " ".repeat(padding),
            Style::default().bg(WARM_GRAY_BG),
        ));
        spans.push(Span::styled(
            pos_text,
            Style::default().fg(Color::Cyan).bg(WARM_GRAY_BG),
        ));
    }

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).style(Style::default().bg(WARM_GRAY_BG));
    f.render_widget(paragraph, area);
}
