use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::tools::ToolKind;

// Warm accent colors (shared with mod.rs)
const WARM_GOLDEN: Color = Color::Indexed(220);
const WARM_ORANGE: Color = Color::Indexed(214);
const WARM_GRAY_DIM: Color = Color::Indexed(243);
const WARM_GRAY_SEP: Color = Color::Indexed(239);

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(Span::styled(
        " Tools",
        Style::default().fg(WARM_ORANGE).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    for tool in ToolKind::ALL {
        let is_active = app.active_tool == tool;
        let prefix = if is_active { "\u{25B8}" } else { " " }; // ▸ or space
        let style = if is_active {
            Style::default()
                .fg(Color::Black)
                .bg(WARM_GOLDEN)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        lines.push(Line::from(Span::styled(
            format!(" {}{} {} {}", prefix, tool.key(), tool.icon(), tool.name()),
            style,
        )));
    }

    // Separator
    let sep_width = area.width.min(12) as usize;
    lines.push(Line::from(Span::styled(
        format!(" {}", "\u{2500}".repeat(sep_width.saturating_sub(2))),
        Style::default().fg(WARM_GRAY_SEP),
    )));

    lines.push(Line::from(Span::styled(
        " Symmetry",
        Style::default().fg(WARM_ORANGE).add_modifier(Modifier::BOLD),
    )));

    let sym = app.symmetry;
    let h_style = if sym.has_horizontal() {
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(WARM_GRAY_DIM)
    };
    let v_style = if sym.has_vertical() {
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(WARM_GRAY_DIM)
    };

    lines.push(Line::from(vec![
        Span::styled(" [H] ", h_style),
        Span::styled("[V]", v_style),
    ]));

    // Separator
    lines.push(Line::from(Span::styled(
        format!(" {}", "\u{2500}".repeat(sep_width.saturating_sub(2))),
        Style::default().fg(WARM_GRAY_SEP),
    )));

    lines.push(Line::from(Span::styled(
        " Grid",
        Style::default().fg(WARM_ORANGE).add_modifier(Modifier::BOLD),
    )));
    let grid_text = if app.show_grid { " [G] On" } else { " [G] Off" };
    let grid_style = if app.show_grid {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(WARM_GRAY_DIM)
    };
    lines.push(Line::from(Span::styled(grid_text, grid_style)));

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, area);
}
