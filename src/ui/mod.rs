pub mod editor;
pub mod toolbar;
pub mod palette;
pub mod statusbar;

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};

use crate::app::{App, AppMode};
use crate::input::CanvasArea;
use crate::theme::Theme;

/// Render the full UI and return the canvas area for mouse mapping.
pub fn render(f: &mut Frame, app: &App) -> CanvasArea {
    let size = f.area();
    let theme = app.theme();

    // Check minimum size
    if size.width < 100 || size.height < 36 {
        let lines = vec![
            ratatui::text::Line::from(""),
            ratatui::text::Line::from(ratatui::text::Span::styled(
                "\u{0295}\u{2022}\u{1d25}\u{2022}\u{0294}",
                Style::default().fg(theme.accent),
            )),
            ratatui::text::Line::from(""),
            ratatui::text::Line::from(ratatui::text::Span::styled(
                "oh no, i'm squished!",
                Style::default().fg(Color::White),
            )),
            ratatui::text::Line::from(""),
            ratatui::text::Line::from(ratatui::text::Span::styled(
                format!("current: {}x{}", size.width, size.height),
                Style::default().fg(theme.dim),
            )),
            ratatui::text::Line::from(ratatui::text::Span::styled(
                "need:    100x36",
                Style::default().fg(theme.dim),
            )),
            ratatui::text::Line::from(""),
            ratatui::text::Line::from(ratatui::text::Span::styled(
                "please resize your terminal!",
                Style::default().fg(theme.highlight),
            )),
        ];
        let msg = Paragraph::new(lines).alignment(Alignment::Center);
        f.render_widget(msg, size);
        return CanvasArea {
            left: 0,
            top: 0,
            width: 0,
            height: 0,
            viewport_w: 0,
            viewport_h: 0,
        };
    }

    // Top-level: main bordered frame + status bar outside
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(34),   // Main frame
            Constraint::Length(1), // Status bar (outside border)
        ])
        .split(size);

    let main_area = outer[0];
    let status_area = outer[1];

    // Render the main border frame
    let main_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.separator));
    let inner = main_block.inner(main_area).inner(Margin::new(1, 0));
    f.render_widget(main_block, main_area);

    // Inside the frame: header + body
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header
            Constraint::Min(32),  // Body
        ])
        .split(inner);

    let header_area = vertical[0];
    let body_area = vertical[1];

    // Header
    render_header(f, app, header_area, theme);

    // Body: left toolbar | canvas | right palette
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .spacing(1)
        .constraints([
            Constraint::Length(14), // Toolbar (bordered panel)
            Constraint::Min(60),   // Canvas (reduced for margin+spacing)
            Constraint::Length(20), // Palette (bordered panel)
        ])
        .split(body_area);

    let toolbar_area = horizontal[0];
    let canvas_area = horizontal[1];
    let palette_area = horizontal[2];

    // Toolbar (4 boxes)
    let tool_lines = toolbar::tool_lines(app);
    let sym_lines = toolbar::symmetry_lines(app);
    let blk_lines = toolbar::block_lines(app);
    let clr_lines = toolbar::color_swatch_lines(app);
    render_box_column(f, toolbar_area, &[
        BoxContent { title: " \u{2022} Tools \u{2022} ", lines: &tool_lines },
        BoxContent { title: " \u{2022} Symmetry \u{2022} ", lines: &sym_lines },
        BoxContent { title: " \u{2022} Block \u{2022} ", lines: &blk_lines },
        BoxContent { title: " \u{2022} Active \u{2022} ", lines: &clr_lines },
    ], theme);

    // Canvas — unified zoom-aware renderer
    let canvas_screen_area = editor::render(f, app, canvas_area);

    // Palette (3 boxes)
    let colors_lines = palette::color_lines(app);
    let section_lines = palette::section_lines(app);
    let info_lines = palette::info_lines(app);
    let section_title = if let Some(ref cp) = app.custom_palette {
        format!(" \u{2022} {} \u{2022} ", cp.name)
    } else {
        " \u{2022} Sections \u{2022} ".to_string()
    };
    render_palette_column(
        f, palette_area,
        &colors_lines, &section_lines, &info_lines,
        &section_title, app.palette_scroll, theme,
    );

    // Status bar (outside the border)
    statusbar::render(f, app, status_area);

    // Overlays
    match app.mode {
        AppMode::Help => render_help(f, app, size),
        AppMode::Quitting => render_quit_prompt(f, size),
        AppMode::FileDialog => render_file_dialog(f, app, size),
        AppMode::ExportDialog => render_export_dialog(f, app, size),
        AppMode::SaveAs => render_text_input(f, app, size, "Save As", "Enter project name:"),
        AppMode::ExportFile => render_text_input(f, app, size, "Export", "Enter filename:"),
        AppMode::Recovery => render_recovery_prompt(f, app, size),
        AppMode::ColorSliders => render_color_sliders(f, app, size),
        AppMode::PaletteDialog => render_palette_dialog(f, app, size),
        AppMode::PaletteNameInput => render_text_input(f, app, size, "New Palette", "Enter palette name:"),
        AppMode::PaletteRename => render_text_input(f, app, size, "Rename Palette", "Enter new name:"),
        AppMode::PaletteExport => render_text_input(f, app, size, "Export Palette", "Enter destination path:"),
        AppMode::NewCanvas => render_new_canvas(f, app, size),
        AppMode::ResizeCanvas => render_resize_canvas(f, app, size),
        AppMode::ResizeCropConfirm => render_resize_crop_confirm(f, app, size),
        AppMode::HexColorInput => render_hex_input(f, app, size),
        AppMode::BlockPicker => render_block_picker(f, app, size),
        _ => {}
    }

    canvas_screen_area
}

struct BoxContent<'a> {
    title: &'a str,
    lines: &'a [ratatui::text::Line<'static>],
}

/// Render N bordered boxes evenly distributed vertically in a column.
fn render_box_column(
    f: &mut Frame,
    column: Rect,
    boxes: &[BoxContent],
    theme: &Theme,
) {
    let n = boxes.len() as u16;
    let box_heights: Vec<u16> = boxes.iter()
        .map(|b| b.lines.len() as u16 + 2)
        .collect();
    let total_box_height: u16 = box_heights.iter().sum();
    let remaining = column.height.saturating_sub(total_box_height);
    let gap_count = n + 1;
    let gap = remaining / gap_count.max(1);

    let mut y = column.y + gap;
    for (i, bx) in boxes.iter().enumerate() {
        let h = box_heights[i];
        let area = Rect::new(column.x, y, column.width, h);
        render_bordered_panel(f, area, bx.lines, bx.title, theme);
        y += h + gap;
    }
}

/// Render 3 palette boxes: Colors (fixed), Sections (scrollable), Color info (fixed).
#[allow(clippy::too_many_arguments)]
fn render_palette_column(
    f: &mut Frame,
    column: Rect,
    colors_lines: &[ratatui::text::Line<'static>],
    section_lines: &[ratatui::text::Line<'static>],
    info_lines: &[ratatui::text::Line<'static>],
    section_title: &str,
    scroll: usize,
    theme: &Theme,
) {
    let colors_height = colors_lines.len() as u16 + 2;
    let info_height = info_lines.len() as u16 + 2;
    let gap_count = 4u16; // 3 boxes → 4 gaps
    let section_content_height = section_lines.len() as u16;

    // Sections box gets remaining space after other boxes and gaps
    let section_max = column.height
        .saturating_sub(colors_height + info_height + gap_count);
    let section_box_height = (section_content_height + 2)
        .min(section_max)
        .max(5); // minimum 5 rows (3 headers + border)

    let total_box_height = colors_height + section_box_height + info_height;
    let remaining = column.height.saturating_sub(total_box_height);
    let gap = remaining / gap_count.max(1);

    let mut y = column.y + gap;

    // Colors box
    let colors_area = Rect::new(column.x, y, column.width, colors_height);
    render_bordered_panel(f, colors_area, colors_lines, " \u{2022} Colors \u{2022} ", theme);
    y += colors_height + gap;

    // Sections box (scrollable)
    let section_area = Rect::new(column.x, y, column.width, section_box_height);
    render_bordered_panel_scrollable(f, section_area, section_lines, section_title, scroll, theme);
    y += section_box_height + gap;

    // Color info box
    let info_area = Rect::new(column.x, y, column.width, info_height);
    render_bordered_panel(f, info_area, info_lines, " \u{2022} Color \u{2022} ", theme);
}

/// Render content lines inside a vertically-centered bordered panel.
fn render_bordered_panel(
    f: &mut Frame,
    column: Rect,
    lines: &[ratatui::text::Line<'static>],
    title: &str,
    theme: &Theme,
) {
    let content_height = lines.len() as u16;
    let panel_height = (content_height + 2).min(column.height); // +2 for border
    let offset_y = (column.height.saturating_sub(panel_height)) / 2;

    let panel_area = Rect::new(
        column.x,
        column.y + offset_y,
        column.width,
        panel_height,
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_accent))
        .title(ratatui::text::Span::styled(
            title.to_string(),
            Style::default().fg(theme.border_accent).add_modifier(Modifier::BOLD),
        ));

    let paragraph = Paragraph::new(lines.to_vec()).block(block);
    f.render_widget(paragraph, panel_area);
}

/// Render content lines inside a bordered panel with scroll support.
fn render_bordered_panel_scrollable(
    f: &mut Frame,
    column: Rect,
    lines: &[ratatui::text::Line<'static>],
    title: &str,
    scroll: usize,
    theme: &Theme,
) {
    let content_height = lines.len() as u16;
    let inner_height = column.height.saturating_sub(2); // available inside border

    let (panel_area, scroll_offset) = if content_height <= inner_height {
        // Content fits — center the panel
        let panel_height = content_height + 2;
        let offset_y = (column.height.saturating_sub(panel_height)) / 2;
        (
            Rect::new(column.x, column.y + offset_y, column.width, panel_height),
            0u16,
        )
    } else {
        // Content overflows — fill column, apply scroll
        (column, scroll as u16)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_accent))
        .title(ratatui::text::Span::styled(
            title.to_string(),
            Style::default().fg(theme.border_accent).add_modifier(Modifier::BOLD),
        ));

    let paragraph = Paragraph::new(lines.to_vec())
        .block(block)
        .scroll((scroll_offset, 0));
    f.render_widget(paragraph, panel_area);
}

fn render_header(f: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let name = app
        .project_name
        .as_deref()
        .unwrap_or("untitled");
    let dirty_marker = if app.dirty { "*" } else { "" };
    let tool_name = app.active_tool.name();
    let sym = app.symmetry.label();

    let header_text = format!(
        " \u{0295}\u{2022}\u{1d25}\u{2022}\u{0294} kakukuma \u{2014} {}{} {:>width$}",
        name,
        dirty_marker,
        format!("Tool: {}  Sym: {}", tool_name, sym),
        width = (area.width as usize).saturating_sub(name.len() + dirty_marker.len() + 22)
    );

    let header = Paragraph::new(header_text)
        .style(Style::default().fg(Color::White).bg(theme.header_bg));
    f.render_widget(header, area);
}

fn render_help(f: &mut Frame, app: &App, area: Rect) {
    use ratatui::text::Span;
    let theme = app.theme();

    let sep = Style::default().fg(theme.separator).bg(theme.panel_bg);
    let hdr = Style::default().fg(theme.accent).bg(theme.panel_bg);
    let txt = Style::default().fg(Color::White).bg(theme.panel_bg);
    let dim = Style::default().fg(theme.dim).bg(theme.panel_bg);

    let lines: Vec<ratatui::text::Line> = vec![
        ratatui::text::Line::from(""),
        ratatui::text::Line::from(vec![
            Span::styled("  Tools", hdr),
            Span::styled("                Canvas", hdr),
        ]),
        ratatui::text::Line::from(vec![
            Span::styled("  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}", sep),
            Span::styled("                \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}", sep),
        ]),
        ratatui::text::Line::from(vec![
            Span::styled("  P  Pencil", txt),
            Span::styled("         WASD  Move cursor", txt),
        ]),
        ratatui::text::Line::from(vec![
            Span::styled("  E  Eraser", txt),
            Span::styled("         Space Draw at cursor", txt),
        ]),
        ratatui::text::Line::from(vec![
            Span::styled("  L  Line", txt),
            Span::styled("           Mouse Click/drag", txt),
        ]),
        ratatui::text::Line::from(vec![
            Span::styled("  R  Rectangle", txt),
            Span::styled("      Z    Cycle zoom (1x/2x/4x)", txt),
        ]),
        ratatui::text::Line::from(vec![
            Span::styled("  F  Fill", txt),
            Span::styled("           B    Cycle block", txt),
        ]),
        ratatui::text::Line::from(vec![
            Span::styled("  I  Eyedropper", txt),
            Span::styled("     \u{21E7}B   Block picker", txt),
        ]),
        ratatui::text::Line::from(vec![
            Span::styled("                    ", txt),
            Span::styled("G    Cycle shade (\u{2591}\u{2592}\u{2593})", txt),
        ]),
        ratatui::text::Line::from(vec![
            Span::styled("                    ", txt),
            Span::styled("T    Rect fill/outline", txt),
        ]),
        ratatui::text::Line::from(""),
        ratatui::text::Line::from(vec![
            Span::styled("  Colors", hdr),
            Span::styled("            Symmetry", hdr),
        ]),
        ratatui::text::Line::from(vec![
            Span::styled("  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}", sep),
            Span::styled("            \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}", sep),
        ]),
        ratatui::text::Line::from(vec![
            Span::styled("  1-0  Quick pick", txt),
            Span::styled("   H  Horizontal mirror", txt),
        ]),
        ratatui::text::Line::from(vec![
            Span::styled("  S    HSL sliders", txt),
            Span::styled("  V  Vertical mirror", txt),
        ]),
        ratatui::text::Line::from(Span::styled("  X    Hex color input", txt)),
        ratatui::text::Line::from(vec![
            Span::styled("  A    Add color", txt),
            Span::styled("    File", hdr),
        ]),
        ratatui::text::Line::from(vec![
            Span::styled("  C    Palettes", txt),
            Span::styled("     \u{2500}\u{2500}\u{2500}\u{2500}", sep),
        ]),
        ratatui::text::Line::from(vec![
            Span::styled("                    ", txt),
            Span::styled("^S Save  ^O Open", txt),
        ]),
        ratatui::text::Line::from(vec![
            Span::styled("  Palette", hdr),
            Span::styled("           ^N New   ^E Export", txt),
        ]),
        ratatui::text::Line::from(vec![
            Span::styled("  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}", sep),
            Span::styled("           ^Z Undo  ^Y Redo", txt),
        ]),
        ratatui::text::Line::from(vec![
            Span::styled("  \u{2191}\u{2193}\u{2190}\u{2192} Browse", txt),
            Span::styled("        ^T Theme", txt),
        ]),
        ratatui::text::Line::from(vec![
            Span::styled("  Enter  Select/Toggle", txt),
            Span::styled("  Q Quit  ? Help", txt),
        ]),
        ratatui::text::Line::from(""),
        ratatui::text::Line::from(Span::styled(
            "         Press any key to close",
            dim,
        )),
    ];

    let width = 48;
    let height = lines.len() as u16 + 2;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let help_area = Rect::new(x, y, width, height);

    let help = Paragraph::new(lines)
        .style(Style::default().fg(Color::White).bg(theme.panel_bg))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Help ")
                .style(Style::default().fg(Color::White).bg(theme.panel_bg)),
        );
    f.render_widget(Clear, help_area);
    f.render_widget(help, help_area);
}

fn render_quit_prompt(f: &mut Frame, area: Rect) {
    let width = 40;
    let height = 5;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let prompt_area = Rect::new(x, y, width, height);

    let prompt = Paragraph::new(" Unsaved changes. Quit? (y/n)")
        .style(Style::default().fg(Color::White).bg(Color::Red))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Quit ")
                .style(Style::default().fg(Color::White).bg(Color::Red)),
        );
    f.render_widget(Clear, prompt_area);
    f.render_widget(prompt, prompt_area);
}

fn render_file_dialog(f: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();
    let file_count = app.file_dialog_files.len();
    let height = (file_count as u16 + 4).min(20);
    let width = 44;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let dialog_area = Rect::new(x, y, width, height);

    let mut lines: Vec<ratatui::text::Line> = Vec::new();
    let visible_start = if app.file_dialog_selected > (height as usize).saturating_sub(5) {
        app.file_dialog_selected - (height as usize).saturating_sub(5)
    } else {
        0
    };

    for (i, filename) in app.file_dialog_files.iter().enumerate().skip(visible_start) {
        if lines.len() >= (height as usize).saturating_sub(4) {
            break;
        }
        let is_selected = i == app.file_dialog_selected;
        let prefix = if is_selected { "> " } else { "  " };
        let style = if is_selected {
            Style::default().fg(Color::Black).bg(theme.highlight)
        } else {
            Style::default().fg(Color::White).bg(theme.panel_bg)
        };
        lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
            format!("{}{}", prefix, filename),
            style,
        )));
    }

    lines.push(ratatui::text::Line::from(""));
    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        " \u{2191}\u{2193} Navigate  Enter Open  Esc Cancel",
        Style::default().fg(theme.dim).bg(theme.panel_bg),
    )));

    let dialog = Paragraph::new(lines)
        .style(Style::default().fg(Color::White).bg(theme.panel_bg))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Open File ")
                .style(Style::default().fg(Color::White).bg(theme.panel_bg)),
        );
    f.render_widget(Clear, dialog_area);
    f.render_widget(dialog, dialog_area);
}

fn render_export_dialog(f: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();
    let is_colored = app.export_format == 1;
    let width = 42;
    let height = if is_colored { 17 } else { 12 };
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let dialog_area = Rect::new(x, y, width, height);

    let format_opts = ["Plain", "Colored"];
    let color_fmt_opts = ["24-bit RGB", "256 color", "16 color"];
    let dest_opts = ["Clipboard", "File"];

    let dim_style = Style::default().fg(theme.dim).bg(theme.panel_bg);

    let mut lines: Vec<ratatui::text::Line> = Vec::new();

    // Format row (cursor == 0)
    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        " Format:",
        Style::default().fg(theme.accent).bg(theme.panel_bg),
    )));
    let mut fmt_spans = Vec::new();
    fmt_spans.push(ratatui::text::Span::raw("  "));
    for (i, opt) in format_opts.iter().enumerate() {
        let selected = i == app.export_format;
        let focused = app.export_cursor == 0;
        let style = if selected && focused {
            Style::default().fg(Color::Indexed(16)).bg(theme.highlight)
        } else if selected {
            Style::default().fg(Color::Indexed(16)).bg(Color::Gray)
        } else {
            Style::default().fg(Color::White).bg(theme.panel_bg)
        };
        fmt_spans.push(ratatui::text::Span::styled(format!(" {} ", opt), style));
        if i == 0 {
            fmt_spans.push(ratatui::text::Span::raw(" "));
        }
    }
    lines.push(ratatui::text::Line::from(fmt_spans));

    // Format description
    let fmt_desc = if is_colored {
        "  Blocks with ANSI color codes"
    } else {
        "  Block characters only, no color"
    };
    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(fmt_desc, dim_style)));
    lines.push(ratatui::text::Line::from(""));

    // Color format row (cursor == 1, only when Colored)
    if is_colored {
        lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
            " Color depth:",
            Style::default().fg(theme.accent).bg(theme.panel_bg),
        )));
        let mut cf_spans = Vec::new();
        cf_spans.push(ratatui::text::Span::raw("  "));
        for (i, opt) in color_fmt_opts.iter().enumerate() {
            let selected = i == app.export_color_format;
            let focused = app.export_cursor == 1;
            let style = if selected && focused {
                Style::default().fg(Color::Indexed(16)).bg(theme.highlight)
            } else if selected {
                Style::default().fg(Color::Indexed(16)).bg(Color::Gray)
            } else {
                Style::default().fg(Color::White).bg(theme.panel_bg)
            };
            cf_spans.push(ratatui::text::Span::styled(format!(" {} ", opt), style));
            if i < color_fmt_opts.len() - 1 {
                cf_spans.push(ratatui::text::Span::raw(" "));
            }
        }
        lines.push(ratatui::text::Line::from(cf_spans));

        // Color format description
        let cf_desc = match app.export_color_format {
            0 => "  Best quality \u{2014} modern terminals",
            1 => "  Good compat \u{2014} most terminals",
            _ => "  Max compat \u{2014} all terminals",
        };
        lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(cf_desc, dim_style)));
        lines.push(ratatui::text::Line::from(""));
    }

    // Destination row (cursor == 1 for Plain, cursor == 2 for Colored)
    let dest_cursor = if is_colored { 2 } else { 1 };
    let ext = if is_colored { ".ans" } else { ".txt" };
    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        format!(" Destination ({}):", ext),
        Style::default().fg(theme.accent).bg(theme.panel_bg),
    )));
    let mut dest_spans = Vec::new();
    dest_spans.push(ratatui::text::Span::raw("  "));
    for (i, opt) in dest_opts.iter().enumerate() {
        let selected = i == app.export_dest;
        let focused = app.export_cursor == dest_cursor;
        let style = if selected && focused {
            Style::default().fg(Color::Indexed(16)).bg(theme.highlight)
        } else if selected {
            Style::default().fg(Color::Indexed(16)).bg(Color::Gray)
        } else {
            Style::default().fg(Color::White).bg(theme.panel_bg)
        };
        dest_spans.push(ratatui::text::Span::styled(format!(" {} ", opt), style));
        if i == 0 {
            dest_spans.push(ratatui::text::Span::raw(" "));
        }
    }
    lines.push(ratatui::text::Line::from(dest_spans));
    lines.push(ratatui::text::Line::from(""));

    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        " \u{2191}\u{2193} Row  \u{2190}\u{2192} Option  Enter Go  Esc Cancel",
        Style::default().fg(theme.dim).bg(theme.panel_bg),
    )));

    let dialog = Paragraph::new(lines)
        .style(Style::default().fg(Color::White).bg(theme.panel_bg))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Export ")
                .style(Style::default().fg(Color::White).bg(theme.panel_bg)),
        );
    f.render_widget(Clear, dialog_area);
    f.render_widget(dialog, dialog_area);
}

fn render_text_input(f: &mut Frame, app: &App, area: Rect, title: &str, prompt: &str) {
    let theme = app.theme();
    let width = 44;
    let height = 7;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let dialog_area = Rect::new(x, y, width, height);

    let mut lines: Vec<ratatui::text::Line> = Vec::new();
    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        format!(" {}", prompt),
        Style::default().fg(theme.accent).bg(theme.panel_bg),
    )));
    lines.push(ratatui::text::Line::from(""));
    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        format!(" {}\u{2588}", app.text_input),
        Style::default().fg(Color::White).bg(Color::Black),
    )));
    lines.push(ratatui::text::Line::from(""));
    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        " Enter Confirm  Esc Cancel",
        Style::default().fg(theme.dim).bg(theme.panel_bg),
    )));

    let dialog = Paragraph::new(lines)
        .style(Style::default().fg(Color::White).bg(theme.panel_bg))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(format!(" {} ", title))
                .style(Style::default().fg(Color::White).bg(theme.panel_bg)),
        );
    f.render_widget(Clear, dialog_area);
    f.render_widget(dialog, dialog_area);
}

fn render_recovery_prompt(f: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();
    let width = 44;
    let height = 5;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let prompt_area = Rect::new(x, y, width, height);

    let prompt = Paragraph::new(" Autosave found. Recover? (y/n)")
        .style(Style::default().fg(Color::White).bg(theme.border_accent))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Recovery ")
                .style(Style::default().fg(Color::White).bg(theme.border_accent)),
        );
    f.render_widget(Clear, prompt_area);
    f.render_widget(prompt, prompt_area);
}

fn render_color_sliders(f: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();
    let width = 44;
    let height = 15;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let dialog_area = Rect::new(x, y, width, height);

    let bar_width = 20;
    let sliders: [(&str, u16, u16); 3] = [
        ("H", app.slider_h, 359),
        ("S", app.slider_s as u16, 100),
        ("L", app.slider_l as u16, 100),
    ];

    let mut lines: Vec<ratatui::text::Line> = Vec::new();

    for (i, (label, value, max_val)) in sliders.iter().enumerate() {
        let is_active = i as u8 == app.slider_active;
        let filled = (*value as usize * bar_width) / (*max_val as usize).max(1);
        let empty = bar_width - filled;
        let bar: String = format!(
            "{}{}",
            "\u{2588}".repeat(filled),
            "\u{2591}".repeat(empty),
        );

        let label_style = if is_active {
            Style::default().fg(theme.accent).add_modifier(ratatui::style::Modifier::BOLD)
        } else {
            Style::default().fg(theme.dim)
        };

        let bar_style = if is_active {
            Style::default().fg(Color::White).bg(theme.panel_bg)
        } else {
            Style::default().fg(theme.dim).bg(theme.panel_bg)
        };

        lines.push(ratatui::text::Line::from(vec![
            ratatui::text::Span::styled(format!(" {} ", label), label_style),
            ratatui::text::Span::styled(bar, bar_style),
            ratatui::text::Span::styled(
                format!(" {:>3}", value),
                Style::default().fg(Color::White).bg(theme.panel_bg),
            ),
        ]));
    }

    lines.push(ratatui::text::Line::from(""));

    // Live preview
    let (r, g, b) = crate::palette::hsl_to_rgb(app.slider_h, app.slider_s, app.slider_l);
    let preview_color = crate::palette::nearest_color(r, g, b);
    let preview_rcolor = preview_color.to_ratatui();
    let idx_256 = crate::cell::nearest_256(&preview_color);

    lines.push(ratatui::text::Line::from(vec![
        ratatui::text::Span::styled(" Preview: ", Style::default().fg(theme.dim).bg(theme.panel_bg)),
        ratatui::text::Span::styled(
            "\u{2588}\u{2588}\u{2588}\u{2588}",
            Style::default().fg(preview_rcolor).bg(theme.panel_bg),
        ),
        ratatui::text::Span::styled(
            format!("  {}", preview_color.name()),
            Style::default().fg(theme.dim).bg(theme.panel_bg),
        ),
    ]));

    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        format!(" RGB: ({}, {}, {})", r, g, b),
        Style::default().fg(theme.dim).bg(theme.panel_bg),
    )));

    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        format!(" Hex: {}  Index: {}", preview_color.name(), idx_256),
        Style::default().fg(theme.dim).bg(theme.panel_bg),
    )));

    lines.push(ratatui::text::Line::from(""));
    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        " \u{2191}\u{2193} Slider  \u{2190}\u{2192} Adjust  Enter Apply  Esc Cancel",
        Style::default().fg(theme.dim).bg(theme.panel_bg),
    )));

    let dialog = Paragraph::new(lines)
        .style(Style::default().fg(Color::White).bg(theme.panel_bg))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Color Sliders ")
                .style(Style::default().fg(Color::White).bg(theme.panel_bg)),
        );
    f.render_widget(Clear, dialog_area);
    f.render_widget(dialog, dialog_area);
}

fn render_palette_dialog(f: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();
    let file_count = app.palette_dialog_files.len();
    let height = (file_count as u16 + 8).min(22);
    let width = 44;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let dialog_area = Rect::new(x, y, width, height);

    let mut lines: Vec<ratatui::text::Line> = Vec::new();

    if app.palette_dialog_files.is_empty() {
        lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
            " No palettes found",
            Style::default().fg(theme.dim).bg(theme.panel_bg),
        )));
    } else {
        let visible_start = if app.palette_dialog_selected > (height as usize).saturating_sub(7) {
            app.palette_dialog_selected - (height as usize).saturating_sub(7)
        } else {
            0
        };

        for (i, filename) in app.palette_dialog_files.iter().enumerate().skip(visible_start) {
            if lines.len() >= (height as usize).saturating_sub(6) {
                break;
            }
            let is_selected = i == app.palette_dialog_selected;
            let prefix = if is_selected { "> " } else { "  " };
            let style = if is_selected {
                Style::default().fg(Color::Black).bg(theme.highlight)
            } else {
                Style::default().fg(Color::White).bg(theme.panel_bg)
            };
            lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
                format!("{}{}", prefix, filename),
                style,
            )));
        }
    }

    // Show active palette
    if let Some(ref cp) = app.custom_palette {
        lines.push(ratatui::text::Line::from(""));
        lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
            format!(" Active: {} ({} colors)", cp.name, cp.colors.len()),
            Style::default().fg(theme.accent).bg(theme.panel_bg),
        )));
    }

    lines.push(ratatui::text::Line::from(""));
    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        " \u{2191}\u{2193} Nav  Enter Load  N New",
        Style::default().fg(theme.dim).bg(theme.panel_bg),
    )));
    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        " R Rename  U Dup  D Del",
        Style::default().fg(theme.dim).bg(theme.panel_bg),
    )));
    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        " X Export  Esc Close",
        Style::default().fg(theme.dim).bg(theme.panel_bg),
    )));

    let dialog = Paragraph::new(lines)
        .style(Style::default().fg(Color::White).bg(theme.panel_bg))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Custom Palettes ")
                .style(Style::default().fg(Color::White).bg(theme.panel_bg)),
        );
    f.render_widget(Clear, dialog_area);
    f.render_widget(dialog, dialog_area);
}

fn render_hex_input(f: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();
    let width = 40u16;
    let height = 9u16;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let dialog_area = Rect::new(x, y, width, height);

    let mut lines: Vec<ratatui::text::Line> = Vec::new();

    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        " Enter hex color (#RRGGBB):",
        Style::default().fg(theme.accent).bg(theme.panel_bg),
    )));
    lines.push(ratatui::text::Line::from(""));
    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        format!(" {}\u{2588}", app.text_input),
        Style::default().fg(Color::White).bg(Color::Black),
    )));
    lines.push(ratatui::text::Line::from(""));

    // Live preview when input is a valid hex color
    let parsed = crate::cell::parse_hex_color(&app.text_input);
    if let Some(rgb) = parsed {
        let preview_color = crate::palette::nearest_color(rgb.r, rgb.g, rgb.b);
        let preview_rcolor = preview_color.to_ratatui();
        lines.push(ratatui::text::Line::from(vec![
            ratatui::text::Span::styled(
                " Preview: ",
                Style::default().fg(theme.dim).bg(theme.panel_bg),
            ),
            ratatui::text::Span::styled(
                "\u{2588}\u{2588}\u{2588}\u{2588}",
                Style::default().fg(preview_rcolor).bg(theme.panel_bg),
            ),
            ratatui::text::Span::styled(
                format!("  {}", preview_color.name()),
                Style::default().fg(theme.dim).bg(theme.panel_bg),
            ),
        ]));
    } else {
        lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
            " Preview: ----",
            Style::default().fg(theme.dim).bg(theme.panel_bg),
        )));
    }

    lines.push(ratatui::text::Line::from(""));
    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        " Enter Apply  Esc Cancel",
        Style::default().fg(theme.dim).bg(theme.panel_bg),
    )));

    let dialog = Paragraph::new(lines)
        .style(Style::default().fg(Color::White).bg(theme.panel_bg))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Hex Color ")
                .style(Style::default().fg(Color::White).bg(theme.panel_bg)),
        );
    f.render_widget(Clear, dialog_area);
    f.render_widget(dialog, dialog_area);
}

fn render_block_picker(f: &mut Frame, app: &App, area: Rect) {
    use crate::cell::blocks;
    use ratatui::text::{Line, Span};

    let theme = app.theme();
    let width = 38u16;
    let height = 10u16;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let dialog_area = Rect::new(x, y, width.min(area.width), height.min(area.height));

    let labels = [" Primary:    ", " Shades:     ", " Vert Fill:  ", " Horiz Fill: "];
    let categories: [&[char]; 4] = [
        &blocks::PRIMARY,
        &blocks::SHADES,
        &blocks::VERTICAL_FILLS,
        &blocks::HORIZONTAL_FILLS,
    ];

    let mut lines: Vec<Line> = Vec::new();

    for (row_idx, (label, chars)) in labels.iter().zip(categories.iter()).enumerate() {
        let mut spans: Vec<Span> = Vec::new();
        spans.push(Span::styled(
            label.to_string(),
            Style::default().fg(theme.dim).bg(theme.panel_bg),
        ));
        for (col_idx, &ch) in chars.iter().enumerate() {
            let is_selected = row_idx == app.block_picker_row && col_idx == app.block_picker_col;
            let style = if is_selected {
                Style::default().fg(theme.panel_bg).bg(theme.highlight)
            } else {
                Style::default().fg(theme.highlight).bg(theme.panel_bg)
            };
            spans.push(Span::styled(format!("{} ", ch), style));
        }
        lines.push(Line::from(spans));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " \u{2190}\u{2192}\u{2191}\u{2193} Navigate  Enter Select  Esc Cancel",
        Style::default().fg(theme.dim).bg(theme.panel_bg),
    )));

    let dialog = Paragraph::new(lines)
        .style(Style::default().fg(Color::White).bg(theme.panel_bg))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Block Picker ")
                .style(Style::default().fg(Color::White).bg(theme.panel_bg)),
        );
    f.render_widget(Clear, dialog_area);
    f.render_widget(dialog, dialog_area);
}

fn render_new_canvas(f: &mut Frame, app: &App, area: Rect) {
    render_canvas_size_dialog(f, app, area, " New Canvas ", false);
}

fn render_resize_canvas(f: &mut Frame, app: &App, area: Rect) {
    render_canvas_size_dialog(f, app, area, " Resize Canvas ", true);
}

fn render_resize_crop_confirm(f: &mut Frame, app: &App, area: Rect) {
    use ratatui::text::{Line, Span};

    let theme = app.theme();
    let w = 38u16;
    let h = 7u16;
    let dialog_area = Rect::new(
        area.width.saturating_sub(w) / 2,
        area.height.saturating_sub(h) / 2,
        w.min(area.width),
        h.min(area.height),
    );
    f.render_widget(Clear, dialog_area);

    let dim = Style::default().fg(theme.dim);
    let warn = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);

    let lines = vec![
        Line::from(Span::styled(" Content will be cropped!", warn)),
        Line::from(Span::raw("")),
        Line::from(Span::styled(
            format!(" {}x{} \u{2192} {}x{}", app.canvas.width, app.canvas.height, app.new_canvas_width, app.new_canvas_height),
            Style::default().fg(Color::White),
        )),
        Line::from(Span::raw("")),
        Line::from(Span::styled(" Enter=Confirm  Any key=Cancel", dim)),
    ];

    let dialog = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Resize Warning ")
            .style(Style::default().fg(Color::Yellow).bg(theme.panel_bg)),
    );
    f.render_widget(dialog, dialog_area);
}

fn render_canvas_size_dialog(f: &mut Frame, app: &App, area: Rect, title: &str, show_current: bool) {
    use ratatui::text::{Line, Span};

    let theme = app.theme();
    let w = 34u16;
    let h = if show_current { 10u16 } else { 9u16 };
    let dialog_area = Rect::new(
        area.width.saturating_sub(w) / 2,
        area.height.saturating_sub(h) / 2,
        w.min(area.width),
        h.min(area.height),
    );
    f.render_widget(Clear, dialog_area);

    let active_style = Style::default().fg(Color::Black).bg(theme.highlight).add_modifier(Modifier::BOLD);
    let inactive_style = Style::default().fg(Color::White);
    let dim = Style::default().fg(theme.dim);

    // Show the input buffer for the active field, stored value for the other
    let (w_display, h_display) = if app.new_canvas_cursor == 0 {
        (format!("{}_", app.new_canvas_input), format!("{}", app.new_canvas_height))
    } else {
        (format!("{}", app.new_canvas_width), format!("{}_", app.new_canvas_input))
    };

    let w_style = if app.new_canvas_cursor == 0 { active_style } else { inactive_style };
    let h_style = if app.new_canvas_cursor == 1 { active_style } else { inactive_style };

    let mut lines = vec![
        Line::from(vec![
            Span::styled(" Width:  ", dim),
            Span::styled(format!("\u{25C0} {:>4} \u{25B6}", w_display), w_style),
        ]),
        Line::from(Span::raw("")),
        Line::from(vec![
            Span::styled(" Height: ", dim),
            Span::styled(format!("\u{25C0} {:>4} \u{25B6}", h_display), h_style),
        ]),
    ];

    if show_current {
        lines.push(Line::from(Span::raw("")));
        lines.push(Line::from(Span::styled(
            format!(" Current: {}x{}", app.canvas.width, app.canvas.height),
            dim,
        )));
    }

    lines.push(Line::from(Span::raw("")));
    lines.push(Line::from(Span::styled(
        " \u{2190}\u{2192}=\u{00B1}1  Type digits  Tab=switch",
        dim,
    )));
    lines.push(Line::from(Span::styled(" Enter=OK  Esc=Cancel", dim)));

    let dialog = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(title)
            .style(Style::default().fg(theme.accent).bg(theme.panel_bg)),
    );
    f.render_widget(dialog, dialog_area);
}
