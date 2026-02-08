use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::cell::Color256;

// Warm accent colors (shared with mod.rs)
const WARM_ORANGE: Color = Color::Indexed(214);
const WARM_GRAY_DIM: Color = Color::Indexed(243);
const WARM_GRAY_SEP: Color = Color::Indexed(239);

const COLS: usize = 6;

/// Render a row of color swatches (up to COLS per row).
/// Returns the lines added.
fn render_color_row(
    colors: &[u8],
    active_color: Color256,
    flat_offset: usize,
    palette_cursor: usize,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for chunk_start in (0..colors.len()).step_by(COLS) {
        let chunk_end = (chunk_start + COLS).min(colors.len());
        let mut spans = Vec::new();
        spans.push(Span::raw(" "));
        for (i, &idx) in colors[chunk_start..chunk_end].iter().enumerate() {
            let color = Color256(idx);
            let rcolor = color.to_ratatui();
            let flat_pos = flat_offset + chunk_start + i;
            let is_cursor = flat_pos == palette_cursor;
            let is_active = color == active_color;

            let marker = if is_cursor {
                ">>"
            } else {
                "\u{2588}\u{2588}"
            };

            let style = if is_cursor || is_active {
                Style::default()
                    .fg(Color::Black)
                    .bg(rcolor)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(rcolor).bg(Color::Black)
            };

            spans.push(Span::styled(marker.to_string(), style));
            if i + chunk_start < chunk_end - 1 {
                spans.push(Span::raw(" "));
            }
        }
        lines.push(Line::from(spans));
    }
    lines
}

fn separator(width: u16) -> Line<'static> {
    let w = width.min(18) as usize;
    Line::from(Span::styled(
        format!(" {}", "\u{2500}".repeat(w.saturating_sub(2))),
        Style::default().fg(WARM_GRAY_SEP),
    ))
}

fn section_header(title: &str) -> Line<'static> {
    Line::from(Span::styled(
        format!(" {}", title),
        Style::default().fg(WARM_ORANGE).add_modifier(Modifier::BOLD),
    ))
}

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let mut all_lines: Vec<Line> = Vec::new();
    let mut flat_offset: usize = 0;

    // === Recent Colors ===
    if !app.recent_colors.is_empty() {
        all_lines.push(section_header("Recent"));
        let recent_indices: Vec<u8> = app.recent_colors.iter().map(|c| c.0).collect();
        let rows = render_color_row(&recent_indices, app.color, flat_offset, app.palette_cursor);
        flat_offset += recent_indices.len();
        all_lines.extend(rows);
        all_lines.push(separator(area.width));
    }

    // === Standard 16 ===
    all_lines.push(section_header("Standard"));
    let standard: Vec<u8> = (0..16).collect();
    let rows = render_color_row(&standard, app.color, flat_offset, app.palette_cursor);
    flat_offset += 16;
    all_lines.extend(rows);
    all_lines.push(separator(area.width));

    // === Hue Groups ===
    for group in &app.hue_groups {
        if group.colors.is_empty() {
            continue;
        }
        all_lines.push(section_header(group.name));
        let rows = render_color_row(&group.colors, app.color, flat_offset, app.palette_cursor);
        flat_offset += group.colors.len();
        all_lines.extend(rows);
    }
    all_lines.push(separator(area.width));

    // === Grayscale ===
    all_lines.push(section_header("Grays"));
    let grays: Vec<u8> = (232..=255).collect();
    let rows = render_color_row(&grays, app.color, flat_offset, app.palette_cursor);
    // flat_offset += 24; // not needed after last section
    let _ = flat_offset; // suppress unused warning
    all_lines.extend(rows);
    all_lines.push(Line::from(""));

    // === Current Color ===
    let color_style = Style::default()
        .fg(app.color.to_ratatui())
        .bg(Color::Black);

    all_lines.push(Line::from(vec![
        Span::raw(" FG: "),
        Span::styled("\u{2588}\u{2588}\u{2588}\u{2588}", color_style),
        Span::styled(
            format!(" {}", app.color.name()),
            Style::default().fg(WARM_GRAY_DIM),
        ),
    ]));

    // === Hints ===
    all_lines.push(Line::from(Span::styled(
        " \u{2191}\u{2193} Browse  [S]liders [C]ustom",
        Style::default().fg(WARM_GRAY_DIM),
    )));

    // === Preview Toggle ===
    all_lines.push(separator(area.width));
    all_lines.push(section_header("Preview"));
    let preview_text = if app.show_preview {
        " [Tab] On"
    } else {
        " [Tab] Off"
    };
    let preview_style = if app.show_preview {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(WARM_GRAY_DIM)
    };
    all_lines.push(Line::from(Span::styled(
        preview_text.to_string(),
        preview_style,
    )));

    // Apply scrolling — clip to the visible area height
    let visible_height = area.height as usize;
    let scroll = app.palette_scroll.min(all_lines.len().saturating_sub(visible_height));
    let end = (scroll + visible_height).min(all_lines.len());
    let visible_lines: Vec<Line> = if scroll < all_lines.len() {
        all_lines[scroll..end].to_vec()
    } else {
        Vec::new()
    };

    let paragraph = Paragraph::new(visible_lines);
    f.render_widget(paragraph, area);
}
