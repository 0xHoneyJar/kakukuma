pub mod editor;
pub mod toolbar;
pub mod palette;
pub mod statusbar;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use crate::app::{App, AppMode};
use crate::input::CanvasArea;

// Warm accent color constants
const WARM_BROWN: Color = Color::Indexed(130);
const WARM_GOLDEN: Color = Color::Indexed(220);
const WARM_ORANGE: Color = Color::Indexed(214);
const WARM_GRAY_DIM: Color = Color::Indexed(243);
const WARM_GRAY_SEP: Color = Color::Indexed(239);
const WARM_GRAY_BG: Color = Color::Indexed(235);

/// Render the full UI and return the canvas area for mouse mapping.
pub fn render(f: &mut Frame, app: &App) -> CanvasArea {
    let size = f.area();

    // Check minimum size
    if size.width < 90 || size.height < 36 {
        let msg = Paragraph::new(format!(
            "Terminal too small: {}x{}\nMinimum: 90x36\nPlease resize.",
            size.width, size.height
        ))
        .style(Style::default().fg(Color::Red));
        f.render_widget(msg, size);
        return CanvasArea {
            left: 0,
            top: 0,
            width: 0,
            height: 0,
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
        .border_style(Style::default().fg(WARM_GRAY_SEP));
    let inner = main_block.inner(main_area);
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
    render_header(f, app, header_area);

    // Body: left toolbar | canvas | right palette
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(12), // Toolbar (wider for icons)
            Constraint::Min(64),   // Canvas (32 cells * 2 chars)
            Constraint::Length(18), // Palette
        ])
        .split(body_area);

    let toolbar_area = horizontal[0];
    let canvas_area = horizontal[1];
    let palette_area = horizontal[2];

    // Toolbar
    toolbar::render(f, app, toolbar_area);

    // Canvas
    let canvas_screen_area = if app.show_preview && size.width >= 160 {
        let side_by_side = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Ratio(1, 2),
                Constraint::Ratio(1, 2),
            ])
            .split(canvas_area);
        let editor_area = editor::render(f, app, side_by_side[0]);
        editor::render_preview(f, app, side_by_side[1]);
        editor_area
    } else if app.show_preview {
        editor::render_preview(f, app, canvas_area);
        CanvasArea {
            left: 0,
            top: 0,
            width: 0,
            height: 0,
        }
    } else {
        editor::render(f, app, canvas_area)
    };

    // Palette
    palette::render(f, app, palette_area);

    // Status bar (outside the border)
    statusbar::render(f, app, status_area);

    // Overlays
    match app.mode {
        AppMode::Help => render_help(f, size),
        AppMode::Quitting => render_quit_prompt(f, size),
        AppMode::FileDialog => render_file_dialog(f, app, size),
        AppMode::ExportDialog => render_export_dialog(f, app, size),
        AppMode::SaveAs => render_text_input(f, app, size, "Save As", "Enter project name:"),
        AppMode::ExportFile => render_text_input(f, app, size, "Export", "Enter filename:"),
        AppMode::Recovery => render_recovery_prompt(f, size),
        AppMode::ColorSliders => render_color_sliders(f, app, size),
        AppMode::PaletteDialog => render_palette_dialog(f, app, size),
        AppMode::PaletteNameInput => render_text_input(f, app, size, "New Palette", "Enter palette name:"),
        _ => {}
    }

    canvas_screen_area
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
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
        .style(Style::default().fg(Color::White).bg(WARM_BROWN));
    f.render_widget(header, area);
}

fn render_help(f: &mut Frame, area: Rect) {
    let help_text = "\
 Keyboard Shortcuts

 Tools:                 Canvas:
  P  Pencil              G  Toggle grid
  E  Eraser              Tab  Toggle preview
  L  Line                T  Rect fill/outline
  R  Rectangle
  F  Fill               Symmetry:
  I  Eyedropper          H  Horizontal mirror
                          V  Vertical mirror
 Colors:
  1-0  Select color     Palette:
  S  HSL sliders         C  Custom palettes
  A  Add to palette      Right-click  Eyedrop

 Actions:               File:
  Ctrl+Z  Undo           Ctrl+S  Save
  Ctrl+Y  Redo           Ctrl+O  Open
  Esc  Cancel tool       Ctrl+N  New
  ?  This help           Ctrl+E  Export
                          Q  Quit

 Press any key to close";

    let width = 50;
    let height = 24;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let help_area = Rect::new(x, y, width, height);

    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::White).bg(WARM_GRAY_BG))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Help ")
                .style(Style::default().fg(Color::White).bg(WARM_GRAY_BG)),
        );
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
    f.render_widget(prompt, prompt_area);
}

fn render_file_dialog(f: &mut Frame, app: &App, area: Rect) {
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
            Style::default().fg(Color::Black).bg(WARM_GOLDEN)
        } else {
            Style::default().fg(Color::White).bg(WARM_GRAY_BG)
        };
        lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
            format!("{}{}", prefix, filename),
            style,
        )));
    }

    lines.push(ratatui::text::Line::from(""));
    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        " \u{2191}\u{2193} Navigate  Enter Open  Esc Cancel",
        Style::default().fg(WARM_GRAY_DIM).bg(WARM_GRAY_BG),
    )));

    let dialog = Paragraph::new(lines)
        .style(Style::default().fg(Color::White).bg(WARM_GRAY_BG))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Open File ")
                .style(Style::default().fg(Color::White).bg(WARM_GRAY_BG)),
        );
    f.render_widget(dialog, dialog_area);
}

fn render_export_dialog(f: &mut Frame, app: &App, area: Rect) {
    let width = 40;
    let height = 10;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let dialog_area = Rect::new(x, y, width, height);

    let format_opts = ["Plain Text", "ANSI Color"];
    let dest_opts = ["Clipboard", "File"];

    let mut lines: Vec<ratatui::text::Line> = Vec::new();

    // Format row
    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        " Format:",
        Style::default().fg(WARM_ORANGE).bg(WARM_GRAY_BG),
    )));
    let mut fmt_spans = Vec::new();
    fmt_spans.push(ratatui::text::Span::raw("  "));
    for (i, opt) in format_opts.iter().enumerate() {
        let selected = i == app.export_format;
        let focused = app.export_cursor == 0;
        let style = if selected && focused {
            Style::default().fg(Color::Black).bg(WARM_GOLDEN)
        } else if selected {
            Style::default().fg(Color::Black).bg(Color::Gray)
        } else {
            Style::default().fg(Color::White).bg(WARM_GRAY_BG)
        };
        fmt_spans.push(ratatui::text::Span::styled(format!(" {} ", opt), style));
        if i == 0 {
            fmt_spans.push(ratatui::text::Span::raw(" "));
        }
    }
    lines.push(ratatui::text::Line::from(fmt_spans));
    lines.push(ratatui::text::Line::from(""));

    // Destination row
    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        " Destination:",
        Style::default().fg(WARM_ORANGE).bg(WARM_GRAY_BG),
    )));
    let mut dest_spans = Vec::new();
    dest_spans.push(ratatui::text::Span::raw("  "));
    for (i, opt) in dest_opts.iter().enumerate() {
        let selected = i == app.export_dest;
        let focused = app.export_cursor == 1;
        let style = if selected && focused {
            Style::default().fg(Color::Black).bg(WARM_GOLDEN)
        } else if selected {
            Style::default().fg(Color::Black).bg(Color::Gray)
        } else {
            Style::default().fg(Color::White).bg(WARM_GRAY_BG)
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
        Style::default().fg(WARM_GRAY_DIM).bg(WARM_GRAY_BG),
    )));

    let dialog = Paragraph::new(lines)
        .style(Style::default().fg(Color::White).bg(WARM_GRAY_BG))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Export ")
                .style(Style::default().fg(Color::White).bg(WARM_GRAY_BG)),
        );
    f.render_widget(dialog, dialog_area);
}

fn render_text_input(f: &mut Frame, app: &App, area: Rect, title: &str, prompt: &str) {
    let width = 44;
    let height = 7;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let dialog_area = Rect::new(x, y, width, height);

    let mut lines: Vec<ratatui::text::Line> = Vec::new();
    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        format!(" {}", prompt),
        Style::default().fg(WARM_ORANGE).bg(WARM_GRAY_BG),
    )));
    lines.push(ratatui::text::Line::from(""));
    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        format!(" {}\u{2588}", app.text_input),
        Style::default().fg(Color::White).bg(Color::Black),
    )));
    lines.push(ratatui::text::Line::from(""));
    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        " Enter Confirm  Esc Cancel",
        Style::default().fg(WARM_GRAY_DIM).bg(WARM_GRAY_BG),
    )));

    let dialog = Paragraph::new(lines)
        .style(Style::default().fg(Color::White).bg(WARM_GRAY_BG))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(format!(" {} ", title))
                .style(Style::default().fg(Color::White).bg(WARM_GRAY_BG)),
        );
    f.render_widget(dialog, dialog_area);
}

fn render_recovery_prompt(f: &mut Frame, area: Rect) {
    let width = 44;
    let height = 5;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let prompt_area = Rect::new(x, y, width, height);

    let prompt = Paragraph::new(" Autosave found. Recover? (y/n)")
        .style(Style::default().fg(Color::White).bg(WARM_BROWN))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Recovery ")
                .style(Style::default().fg(Color::White).bg(WARM_BROWN)),
        );
    f.render_widget(prompt, prompt_area);
}

fn render_color_sliders(f: &mut Frame, app: &App, area: Rect) {
    let width = 44;
    let height = 14;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let dialog_area = Rect::new(x, y, width, height);

    let bar_width = 20;
    let sliders: [(& str, u16, u16); 3] = [
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
            Style::default().fg(WARM_ORANGE).add_modifier(ratatui::style::Modifier::BOLD)
        } else {
            Style::default().fg(WARM_GRAY_DIM)
        };

        let bar_style = if is_active {
            Style::default().fg(Color::White).bg(WARM_GRAY_BG)
        } else {
            Style::default().fg(WARM_GRAY_DIM).bg(WARM_GRAY_BG)
        };

        lines.push(ratatui::text::Line::from(vec![
            ratatui::text::Span::styled(format!(" {} ", label), label_style),
            ratatui::text::Span::styled(bar, bar_style),
            ratatui::text::Span::styled(
                format!(" {:>3}", value),
                Style::default().fg(Color::White).bg(WARM_GRAY_BG),
            ),
        ]));
    }

    lines.push(ratatui::text::Line::from(""));

    // Live preview
    let (r, g, b) = crate::palette::hsl_to_rgb(app.slider_h, app.slider_s, app.slider_l);
    let preview_color = crate::palette::nearest_color(r, g, b);
    let preview_rcolor = preview_color.to_ratatui();

    lines.push(ratatui::text::Line::from(vec![
        ratatui::text::Span::styled(" Preview: ", Style::default().fg(WARM_GRAY_DIM).bg(WARM_GRAY_BG)),
        ratatui::text::Span::styled(
            "\u{2588}\u{2588}\u{2588}\u{2588}",
            Style::default().fg(preview_rcolor).bg(WARM_GRAY_BG),
        ),
        ratatui::text::Span::styled(
            format!("  {}", preview_color.name()),
            Style::default().fg(WARM_GRAY_DIM).bg(WARM_GRAY_BG),
        ),
    ]));

    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        format!(" RGB: ({}, {}, {})  #{}", r, g, b, preview_color.0),
        Style::default().fg(WARM_GRAY_DIM).bg(WARM_GRAY_BG),
    )));

    lines.push(ratatui::text::Line::from(""));
    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        " \u{2191}\u{2193} Slider  \u{2190}\u{2192} Adjust  Enter Apply  Esc Cancel",
        Style::default().fg(WARM_GRAY_DIM).bg(WARM_GRAY_BG),
    )));

    let dialog = Paragraph::new(lines)
        .style(Style::default().fg(Color::White).bg(WARM_GRAY_BG))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Color Sliders ")
                .style(Style::default().fg(Color::White).bg(WARM_GRAY_BG)),
        );
    f.render_widget(dialog, dialog_area);
}

fn render_palette_dialog(f: &mut Frame, app: &App, area: Rect) {
    let file_count = app.palette_dialog_files.len();
    let height = (file_count as u16 + 6).min(20);
    let width = 44;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let dialog_area = Rect::new(x, y, width, height);

    let mut lines: Vec<ratatui::text::Line> = Vec::new();

    if app.palette_dialog_files.is_empty() {
        lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
            " No palettes found",
            Style::default().fg(WARM_GRAY_DIM).bg(WARM_GRAY_BG),
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
                Style::default().fg(Color::Black).bg(WARM_GOLDEN)
            } else {
                Style::default().fg(Color::White).bg(WARM_GRAY_BG)
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
            Style::default().fg(WARM_ORANGE).bg(WARM_GRAY_BG),
        )));
    }

    lines.push(ratatui::text::Line::from(""));
    lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
        " \u{2191}\u{2193} Nav  Enter Load  N New  D Del  Esc Close",
        Style::default().fg(WARM_GRAY_DIM).bg(WARM_GRAY_BG),
    )));

    let dialog = Paragraph::new(lines)
        .style(Style::default().fg(Color::White).bg(WARM_GRAY_BG))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Custom Palettes ")
                .style(Style::default().fg(Color::White).bg(WARM_GRAY_BG)),
        );
    f.render_widget(dialog, dialog_area);
}
