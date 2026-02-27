use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::app::App;
use crate::cell::Rgb;
use crate::palette::{PaletteItem, PaletteSection};
use crate::theme::Theme;

const COLS: usize = 6;
const PALETTE_INNER_WIDTH: usize = 18; // box width (20) minus 2 border chars

/// Render a row of color swatches (up to COLS per row).
fn render_color_row(
    colors: &[Rgb],
    active_color: Rgb,
    flat_offset: usize,
    palette_cursor: usize,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for chunk_start in (0..colors.len()).step_by(COLS) {
        let chunk_end = (chunk_start + COLS).min(colors.len());
        let mut spans = Vec::new();
        let chunk_len = chunk_end - chunk_start;
        let content_width = chunk_len * 2 + chunk_len.saturating_sub(1); // swatches + separators
        let pad = PALETTE_INNER_WIDTH.saturating_sub(content_width) / 2;
        spans.push(Span::raw(" ".repeat(pad.max(1))));
        for (i, &color) in colors[chunk_start..chunk_end].iter().enumerate() {
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
                    .fg(Color::Indexed(16))
                    .bg(rcolor)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(rcolor)
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

/// Render a collapsible section header line.
/// `count_hint` overrides the displayed count for dynamic sections (Recent).
fn section_header_line(section: PaletteSection, expanded: bool, is_cursor: bool, theme: &Theme, count_hint: usize) -> Line<'static> {
    let indicator = if expanded { "\u{25BE}" } else { "\u{25B8}" }; // ▾ or ▸
    let (name, count) = match section {
        PaletteSection::Recent => ("Recent", count_hint),
        PaletteSection::Standard => ("Standard", 16),
        PaletteSection::HueGroups => ("Hue Groups", 216),
        PaletteSection::Grayscale => ("Grayscale", 24),
    };
    let raw_text = format!("{} {} ({})", indicator, name, count);
    let pad = PALETTE_INNER_WIDTH.saturating_sub(raw_text.len()) / 2;
    let text = format!("{}{}", " ".repeat(pad.max(1)), raw_text);
    let style = if is_cursor {
        Style::default()
            .fg(Color::Indexed(16))
            .bg(theme.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD)
    };
    Line::from(Span::styled(text, style))
}

/// Find the index of the first SectionHeader in the palette layout.
fn first_section_index(app: &App) -> usize {
    app.palette_layout
        .iter()
        .position(|item| matches!(item, PaletteItem::SectionHeader(_)))
        .unwrap_or(app.palette_layout.len())
}

/// Curated color swatches (items before the first SectionHeader).
pub fn color_lines(app: &App) -> Vec<Line<'static>> {
    let split = first_section_index(app);
    let layout = &app.palette_layout;

    let mut colors: Vec<Rgb> = Vec::new();
    for item in layout.iter().take(split) {
        if let PaletteItem::Color(color) = item {
            colors.push(*color);
        }
    }

    render_color_row(&colors, app.color, 0, app.palette_cursor)
}

/// Section headers + expanded section colors (from first SectionHeader onward).
pub fn section_lines(app: &App) -> Vec<Line<'static>> {
    let theme = app.theme();
    let split = first_section_index(app);
    let layout = &app.palette_layout;
    let mut all_lines: Vec<Line> = Vec::new();

    let mut i = split;
    let mut color_batch: Vec<Rgb> = Vec::new();
    let mut batch_start = 0;

    while i < layout.len() {
        match layout[i] {
            PaletteItem::Color(color) => {
                if color_batch.is_empty() {
                    batch_start = i;
                }
                color_batch.push(color);
                i += 1;
                // Flush at end or if next item is a header
                if i >= layout.len() || matches!(layout[i], PaletteItem::SectionHeader(_)) {
                    let rows = render_color_row(
                        &color_batch,
                        app.color,
                        batch_start,
                        app.palette_cursor,
                    );
                    all_lines.extend(rows);
                    color_batch.clear();
                }
            }
            PaletteItem::SectionHeader(section) => {
                let expanded = match section {
                    PaletteSection::Recent => app.palette_sections.recent_expanded,
                    PaletteSection::Standard => app.palette_sections.standard_expanded,
                    PaletteSection::HueGroups => app.palette_sections.hue_expanded,
                    PaletteSection::Grayscale => app.palette_sections.grayscale_expanded,
                };
                // Count colors following this header (for dynamic count display)
                let count_hint = match section {
                    PaletteSection::Recent => app.recent_colors.len(),
                    PaletteSection::Standard => 16,
                    PaletteSection::HueGroups => 216,
                    PaletteSection::Grayscale => 24,
                };
                let is_cursor = i == app.palette_cursor;
                all_lines.push(section_header_line(section, expanded, is_cursor, theme, count_hint));
                i += 1;
            }
        }
    }

    all_lines
}

/// Center a text string within PALETTE_INNER_WIDTH.
fn center_line(text: &str, style: Style) -> Line<'static> {
    let pad = PALETTE_INNER_WIDTH.saturating_sub(text.len()) / 2;
    Line::from(Span::styled(
        format!("{}{}", " ".repeat(pad.max(1)), text),
        style,
    ))
}

/// Current color swatch + name + hint lines (5 centered lines).
pub fn info_lines(app: &App) -> Vec<Line<'static>> {
    let theme = app.theme();
    let dim = Style::default().fg(theme.dim);
    let color_style = Style::default()
        .bg(app.color.to_ratatui());

    // Line 1: color swatch + name (mixed styles, centered)
    let swatch = "    ";
    let name = format!(" {}", app.color.name());
    let content_len = 4 + name.len(); // 4 chars for swatch display width
    let pad = PALETTE_INNER_WIDTH.saturating_sub(content_len) / 2;
    let line1 = Line::from(vec![
        Span::raw(" ".repeat(pad.max(1))),
        Span::styled(swatch.to_string(), color_style),
        Span::styled(name, dim),
    ]);

    vec![
        line1,
        center_line("\u{2191}\u{2193} Browse", dim),
        center_line("[S]liders", dim),
        center_line("[C]ustom", dim),
        center_line("[A]dd color", dim),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::cell::Rgb;

    fn lines_text(lines: &[Line]) -> String {
        lines.iter().map(|l| {
            l.spans.iter().map(|s| s.content.as_ref()).collect::<String>()
        }).collect::<Vec<_>>().join("\n")
    }

    #[test]
    fn test_recent_section_renders() {
        let mut app = App::new();
        let red = Rgb::new(255, 0, 0);
        app.recent_colors.push(red);
        app.rebuild_palette_layout();
        let lines = section_lines(&app);
        let text = lines_text(&lines);
        assert!(text.contains("Recent"), "Section lines should contain Recent header, got: {}", text);
    }

    #[test]
    fn test_recent_section_collapsible() {
        let mut app = App::new();
        let red = Rgb::new(255, 0, 0);
        app.recent_colors.push(red);
        app.rebuild_palette_layout();

        // Expanded by default — should have color swatches after header
        let lines_expanded = section_lines(&app);
        let expanded_count = lines_expanded.len();

        // Collapse the Recent section
        app.palette_sections.recent_expanded = false;
        app.rebuild_palette_layout();
        let lines_collapsed = section_lines(&app);
        let collapsed_count = lines_collapsed.len();

        // Collapsed should have fewer lines (header only, no swatches)
        assert!(collapsed_count < expanded_count,
            "Collapsed ({}) should have fewer lines than expanded ({})", collapsed_count, expanded_count);

        // Both should still show "Recent" header
        let text = lines_text(&lines_collapsed);
        assert!(text.contains("Recent"), "Collapsed section should still show Recent header");
    }

    #[test]
    fn test_recent_section_empty_not_rendered() {
        let app = App::new();
        let lines = section_lines(&app);
        let text = lines_text(&lines);
        assert!(!text.contains("Recent"), "Empty recent should not render, got: {}", text);
    }
}
