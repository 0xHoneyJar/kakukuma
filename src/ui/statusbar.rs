use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();
    let mut spans = Vec::new();

    // Status message takes priority
    if let Some(ref msg) = app.status_message {
        spans.push(Span::styled(
            format!(" {} ", msg.text),
            Style::default().fg(theme.highlight).bg(theme.panel_bg),
        ));
    } else {
        // Default shortcut hints — dim undo/redo when unavailable
        let undo_fg = if app.history.can_undo() { Color::White } else { theme.dim };
        let undo_label_fg = if app.history.can_undo() { Color::Gray } else { theme.dim };
        let redo_fg = if app.history.can_redo() { Color::White } else { theme.dim };
        let redo_label_fg = if app.history.can_redo() { Color::Gray } else { theme.dim };

        let sep_style = Style::default().fg(theme.separator).bg(theme.panel_bg);

        // Left group: file ops + edit ops + tool + dimensions
        for &(key, label, key_fg, label_fg) in &[
            ("^S", " Save ", Color::White, Color::Gray),
            ("^O", " Open ", Color::White, Color::Gray),
            ("^E", " Export ", Color::White, Color::Gray),
            ("^I", " Import ", Color::White, Color::Gray),
        ] {
            spans.push(Span::styled(key, Style::default().fg(key_fg).bg(theme.panel_bg)));
            spans.push(Span::styled(label, Style::default().fg(label_fg).bg(theme.panel_bg)));
        }

        spans.push(Span::styled(" \u{2502} ", sep_style));

        for &(key, label, key_fg, label_fg) in &[
            ("^Z", " Undo ", undo_fg, undo_label_fg),
            ("^Y", " Redo ", redo_fg, redo_label_fg),
        ] {
            spans.push(Span::styled(key, Style::default().fg(key_fg).bg(theme.panel_bg)));
            spans.push(Span::styled(label, Style::default().fg(label_fg).bg(theme.panel_bg)));
        }

        spans.push(Span::styled(" \u{2502} ", sep_style));

        // Tool name
        spans.push(Span::styled(
            format!("{} ", app.active_tool.name()),
            Style::default().fg(Color::Gray).bg(theme.panel_bg),
        ));

        spans.push(Span::styled("\u{2502} ", sep_style));

        // Canvas dimensions
        spans.push(Span::styled(
            format!("{}\u{00d7}{}", app.canvas.width, app.canvas.height),
            Style::default().fg(theme.dim).bg(theme.panel_bg),
        ));

        // Right group: color swatch, zoom, help, quit, cursor position
        let mut right_spans: Vec<Span> = Vec::new();

        // Active color swatch
        right_spans.push(Span::styled(
            "  ",
            Style::default().bg(app.color.to_ratatui()),
        ));
        right_spans.push(Span::styled(" ", Style::default().bg(theme.panel_bg)));

        // Zoom level
        right_spans.push(Span::styled(
            format!("{}x ", app.zoom),
            Style::default().fg(theme.dim).bg(theme.panel_bg),
        ));

        for &(key, label) in &[("?", " Help "), ("Q", " Quit ")] {
            right_spans.push(Span::styled(key, Style::default().fg(Color::White).bg(theme.panel_bg)));
            right_spans.push(Span::styled(label, Style::default().fg(Color::Gray).bg(theme.panel_bg)));
        }
        if let Some((x, y)) = app.effective_cursor() {
            right_spans.push(Span::styled(
                format!("({},{}) ", x, y),
                Style::default().fg(Color::Cyan).bg(theme.panel_bg),
            ));
        }

        let left_width: usize = spans.iter().map(|s| s.content.len()).sum();
        let right_width: usize = right_spans.iter().map(|s| s.content.len()).sum();
        let padding = (area.width as usize).saturating_sub(left_width + right_width);
        spans.push(Span::styled(
            " ".repeat(padding),
            Style::default().bg(theme.panel_bg),
        ));
        spans.extend(right_spans);
    }

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).style(Style::default().bg(theme.panel_bg));
    f.render_widget(paragraph, area);
}

/// Build status bar spans without rendering (for testing).
pub fn build_spans(app: &App) -> Vec<Span<'static>> {
    let theme = app.theme();
    let mut spans = Vec::new();

    if let Some(ref msg) = app.status_message {
        spans.push(Span::styled(
            format!(" {} ", msg.text),
            Style::default().fg(theme.highlight).bg(theme.panel_bg),
        ));
    } else {
        let undo_fg = if app.history.can_undo() { Color::White } else { theme.dim };
        let undo_label_fg = if app.history.can_undo() { Color::Gray } else { theme.dim };
        let redo_fg = if app.history.can_redo() { Color::White } else { theme.dim };
        let redo_label_fg = if app.history.can_redo() { Color::Gray } else { theme.dim };
        let sep_style = Style::default().fg(theme.separator).bg(theme.panel_bg);

        for &(key, label, key_fg, label_fg) in &[
            ("^S", " Save ", Color::White, Color::Gray),
            ("^O", " Open ", Color::White, Color::Gray),
            ("^E", " Export ", Color::White, Color::Gray),
            ("^I", " Import ", Color::White, Color::Gray),
        ] {
            spans.push(Span::styled(key, Style::default().fg(key_fg).bg(theme.panel_bg)));
            spans.push(Span::styled(label, Style::default().fg(label_fg).bg(theme.panel_bg)));
        }
        spans.push(Span::styled(" \u{2502} ", sep_style));
        for &(key, label, key_fg, label_fg) in &[
            ("^Z", " Undo ", undo_fg, undo_label_fg),
            ("^Y", " Redo ", redo_fg, redo_label_fg),
        ] {
            spans.push(Span::styled(key, Style::default().fg(key_fg).bg(theme.panel_bg)));
            spans.push(Span::styled(label, Style::default().fg(label_fg).bg(theme.panel_bg)));
        }
        spans.push(Span::styled(" \u{2502} ", sep_style));
        spans.push(Span::styled(
            format!("{} ", app.active_tool.name()),
            Style::default().fg(Color::Gray).bg(theme.panel_bg),
        ));
        spans.push(Span::styled("\u{2502} ", sep_style));
        spans.push(Span::styled(
            format!("{}\u{00d7}{}", app.canvas.width, app.canvas.height),
            Style::default().fg(theme.dim).bg(theme.panel_bg),
        ));

        for &(key, label) in &[("?", " Help "), ("Q", " Quit ")] {
            spans.push(Span::styled(key, Style::default().fg(Color::White).bg(theme.panel_bg)));
            spans.push(Span::styled(label, Style::default().fg(Color::Gray).bg(theme.panel_bg)));
        }
        if let Some((x, y)) = app.effective_cursor() {
            spans.push(Span::styled(
                format!("({},{}) ", x, y),
                Style::default().fg(Color::Cyan).bg(theme.panel_bg),
            ));
        }
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::canvas;

    fn spans_text(spans: &[Span]) -> String {
        spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn test_status_bar_shows_dimensions() {
        let app = App::new();
        let text = spans_text(&build_spans(&app));
        let dims = format!("{}\u{00d7}{}", canvas::DEFAULT_WIDTH, canvas::DEFAULT_HEIGHT);
        assert!(text.contains(&dims), "Status bar should contain canvas dimensions, got: {}", text);
    }

    #[test]
    fn test_status_bar_shows_import() {
        let app = App::new();
        let text = spans_text(&build_spans(&app));
        assert!(text.contains("^I"), "Status bar should contain ^I Import shortcut, got: {}", text);
        assert!(text.contains("Import"), "Status bar should contain Import label, got: {}", text);
    }

    #[test]
    fn test_status_bar_dims_undo_redo() {
        let app = App::new();
        let theme = app.theme();
        let spans = build_spans(&app);
        // Find undo key span (^Z)
        let undo_span = spans.iter().find(|s| s.content.as_ref() == "^Z").unwrap();
        // With empty history, undo should be dimmed
        assert_eq!(undo_span.style.fg, Some(theme.dim));
    }

    #[test]
    fn test_status_bar_shows_tool_name() {
        let app = App::new();
        let text = spans_text(&build_spans(&app));
        assert!(text.contains("Pencil"), "Status bar should show tool name, got: {}", text);
    }

    #[test]
    fn test_status_bar_separators() {
        let app = App::new();
        let text = spans_text(&build_spans(&app));
        // Should have │ separators
        assert!(text.contains("\u{2502}"), "Status bar should contain │ separators, got: {}", text);
    }
}
