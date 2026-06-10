use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::app::App;
use crate::tools::ToolKind;

/// Tool list: 6 tool entries.
pub fn tool_lines(app: &App) -> Vec<Line<'static>> {
    let theme = app.theme();
    let mut lines: Vec<Line> = Vec::new();

    for tool in ToolKind::ALL {
        let is_active = app.active_tool == tool;
        let prefix = if is_active { "\u{25B8}" } else { " " }; // ▸ or space
        let style = if is_active {
            Style::default()
                .fg(Color::Indexed(16))
                .bg(theme.highlight)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        lines.push(Line::from(Span::styled(
            format!(" {}{} {} {}", prefix, tool.key(), tool.icon(), tool.name()),
            style,
        )));
    }

    lines
}

/// Symmetry toggle row: [H] [V].
pub fn symmetry_lines(app: &App) -> Vec<Line<'static>> {
    let theme = app.theme();
    let sym = app.symmetry;
    let h_style = if sym.has_horizontal() {
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.dim)
    };
    let v_style = if sym.has_vertical() {
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.dim)
    };

    vec![Line::from(vec![
        Span::styled(" [H] ", h_style),
        Span::styled("[V]", v_style),
    ])]
}

/// Block character panel: shows primary + shade blocks with shortcuts.
pub fn block_lines(app: &App) -> Vec<Line<'static>> {
    use crate::cell::blocks;

    let theme = app.theme();
    let active = app.active_block;
    let dim = Style::default().fg(theme.dim);
    let sel = Style::default()
        .fg(Color::Indexed(16))
        .bg(theme.highlight)
        .add_modifier(Modifier::BOLD);

    // Row 1: Primary blocks with [B] shortcut
    let mut primary = vec![Span::styled(" ", Style::default())];
    for &ch in &blocks::PRIMARY {
        let s = if ch == active { sel } else { dim };
        primary.push(Span::styled(format!("{}", ch), s));
    }
    primary.push(Span::styled(" [B]", dim));

    // Row 2: Shade blocks with [G] shortcut
    let mut shades = vec![Span::styled(" ", Style::default())];
    for &ch in &blocks::SHADES {
        let s = if ch == active { sel } else { dim };
        shades.push(Span::styled(format!("{}", ch), s));
    }
    shades.push(Span::styled("  [G]", dim));

    // Row 3: Picker hint (show active if from fills)
    let in_primary = blocks::PRIMARY.contains(&active);
    let in_shade = blocks::SHADES.contains(&active);
    let picker_line = if !in_primary && !in_shade {
        Line::from(vec![
            Span::styled(" ", Style::default()),
            Span::styled(format!("{}", active), sel),
            Span::styled(" [\u{21E7}B]", dim),
        ])
    } else {
        Line::from(Span::styled(" [\u{21E7}B] More", dim))
    };

    // Row 4: Rect fill/outline toggle
    let rect_text = if app.filled_rect { " [T] Filled" } else { " [T] Outline" };
    let rect_line = Line::from(Span::styled(rect_text, dim));

    vec![
        Line::from(primary),
        Line::from(shades),
        picker_line,
        rect_line,
    ]
}

/// Active color swatch display.
pub fn color_swatch_lines(app: &App) -> Vec<Line<'static>> {
    let theme = app.theme();
    let label = Line::from(Span::styled(
        " Color:",
        Style::default().fg(theme.accent),
    ));
    let swatch = Line::from(vec![
        Span::styled(" ", Style::default()),
        Span::styled(
            "    ",
            Style::default().bg(app.color.to_ratatui()),
        ),
        Span::styled(
            format!(" {}", app.color.name()),
            Style::default().fg(theme.dim),
        ),
    ]);
    vec![label, swatch]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::cell::blocks;

    fn lines_text(lines: &[Line]) -> String {
        lines.iter().map(|l| {
            l.spans.iter().map(|s| s.content.as_ref()).collect::<String>()
        }).collect::<Vec<_>>().join("\n")
    }

    #[test]
    fn test_block_lines_shows_shades() {
        let app = App::new();
        let text = lines_text(&block_lines(&app));
        assert!(text.contains('\u{2591}'), "Block panel should show ░, got: {}", text);
        assert!(text.contains('\u{2592}'), "Block panel should show ▒, got: {}", text);
        assert!(text.contains('\u{2593}'), "Block panel should show ▓, got: {}", text);
    }

    #[test]
    fn test_block_lines_shows_shortcuts() {
        let app = App::new();
        let text = lines_text(&block_lines(&app));
        assert!(text.contains("[B]"), "Block panel should show [B] shortcut, got: {}", text);
        assert!(text.contains("[G]"), "Block panel should show [G] shortcut, got: {}", text);
        assert!(text.contains("[T]"), "Block panel should show [T] shortcut, got: {}", text);
    }

    #[test]
    fn test_block_lines_highlights_active() {
        let mut app = App::new();
        app.active_block = blocks::SHADE_DARK;
        let lines = block_lines(&app);
        // The shade row should contain the active block with highlight style
        let shade_line = &lines[1]; // row index 1 = shades
        let dark_span = shade_line.spans.iter().find(|s| s.content.contains('\u{2593}'));
        assert!(dark_span.is_some(), "Shade row should contain ▓");
        let span = dark_span.unwrap();
        // Active block should have BOLD modifier
        assert!(span.style.add_modifier.contains(Modifier::BOLD),
            "Active block should be bold");
    }

    #[test]
    fn test_block_lines_picker_shows_fill_char() {
        let mut app = App::new();
        // Set active to a vertical fill (not in primary or shades)
        app.active_block = blocks::LOWER_1_4;
        let lines = block_lines(&app);
        let text = lines_text(&lines);
        // Should show the active fill char instead of "More"
        assert!(text.contains('\u{2582}'), "Picker row should show ▂ when it's active, got: {}", text);
        assert!(!text.contains("More"), "Should not show 'More' when a fill is active, got: {}", text);
    }
}
