use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use crate::app::{App, AppMode};
use crate::canvas::Canvas;
use crate::history::{Action, History};
use crate::palette::{PaletteItem, PaletteSection};
use crate::tools::{ToolKind, ToolState};

/// Canvas area position in terminal coordinates.
/// Set by the UI renderer each frame.
pub struct CanvasArea {
    pub left: u16,
    pub top: u16,
    pub width: u16,
    pub height: u16,
    /// Viewport dimensions in canvas cells (set by renderer)
    pub viewport_w: usize,
    pub viewport_h: usize,
}

impl CanvasArea {
    /// Convert screen coordinates to canvas cell coordinates.
    /// Returns None if outside canvas bounds.
    pub fn screen_to_canvas(&self, screen_x: u16, screen_y: u16, zoom: u8, viewport_x: usize, viewport_y: usize) -> Option<(usize, usize)> {
        if screen_x < self.left || screen_y < self.top {
            return None;
        }
        let rel_x = screen_x - self.left;
        let rel_y = screen_y - self.top;
        if rel_x >= self.width || rel_y >= self.height {
            return None;
        }
        let canvas_x = (rel_x / zoom as u16) as usize + viewport_x;
        let canvas_y = match zoom {
            4 => (rel_y / 2) as usize + viewport_y,
            _ => rel_y as usize + viewport_y,
        };
        Some((canvas_x, canvas_y))
    }
}

pub fn handle_event(app: &mut App, event: Event, canvas_area: &CanvasArea) {
    match app.mode {
        AppMode::Help => {
            // Any key dismisses help
            if matches!(event, Event::Key(_)) {
                app.mode = AppMode::Normal;
            }
            return;
        }
        AppMode::Quitting => {
            if let Event::Key(KeyEvent { code, .. }) = event {
                match code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        app.running = false;
                    }
                    _ => {
                        app.mode = AppMode::Normal;
                    }
                }
            }
            return;
        }
        AppMode::Recovery => {
            if let Event::Key(KeyEvent { code, .. }) = event {
                match code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        app.recover_autosave();
                    }
                    _ => {
                        app.recovery_path = None;
                        app.mode = AppMode::Normal;
                    }
                }
            }
            return;
        }
        AppMode::FileDialog => {
            if let Event::Key(KeyEvent { code, .. }) = event {
                handle_file_dialog(app, code);
            }
            return;
        }
        AppMode::ExportDialog => {
            if let Event::Key(KeyEvent { code, .. }) = event {
                handle_export_dialog(app, code);
            }
            return;
        }
        AppMode::SaveAs => {
            if let Event::Key(key) = event {
                handle_text_input(app, key, TextInputPurpose::SaveAs);
            }
            return;
        }
        AppMode::ExportFile => {
            if let Event::Key(key) = event {
                handle_text_input(app, key, TextInputPurpose::ExportFile);
            }
            return;
        }
        AppMode::ColorSliders => {
            if let Event::Key(KeyEvent { code, .. }) = event {
                handle_color_sliders(app, code);
            }
            return;
        }
        AppMode::PaletteDialog => {
            if let Event::Key(KeyEvent { code, .. }) = event {
                handle_palette_dialog(app, code);
            }
            return;
        }
        AppMode::PaletteNameInput => {
            if let Event::Key(key) = event {
                handle_text_input(app, key, TextInputPurpose::PaletteName);
            }
            return;
        }
        AppMode::PaletteRename => {
            if let Event::Key(key) = event {
                handle_text_input(app, key, TextInputPurpose::PaletteRename);
            }
            return;
        }
        AppMode::PaletteExport => {
            if let Event::Key(key) = event {
                handle_text_input(app, key, TextInputPurpose::PaletteExport);
            }
            return;
        }
        AppMode::NewCanvas => {
            if let Event::Key(KeyEvent { code, .. }) = event {
                handle_new_canvas(app, code);
            }
            return;
        }
        AppMode::ResizeCanvas => {
            if let Event::Key(KeyEvent { code, .. }) = event {
                handle_resize_canvas(app, code);
            }
            return;
        }
        AppMode::ResizeCropConfirm => {
            if let Event::Key(KeyEvent { code, .. }) = event {
                handle_resize_crop_confirm(app, code);
            }
            return;
        }
        AppMode::HexColorInput => {
            if let Event::Key(key) = event {
                handle_hex_input(app, key);
            }
            return;
        }
        AppMode::BlockPicker => {
            if let Event::Key(key) = event {
                handle_block_picker(app, key);
            }
            return;
        }
        AppMode::ImportBrowse => {
            if let Event::Key(KeyEvent { code, .. }) = event {
                handle_import_browse(app, code);
            }
            return;
        }
        AppMode::ImportOptions => {
            if let Event::Key(KeyEvent { code, .. }) = event {
                handle_import_options(app, code);
            }
            return;
        }
        _ => {}
    }

    match event {
        Event::Key(key) => handle_key(app, key),
        Event::Mouse(mouse) => handle_mouse(app, mouse, canvas_area),
        Event::Resize(_, _) => {} // Layout handles this automatically
        _ => {}
    }
}

fn handle_key(app: &mut App, key: KeyEvent) {
    // Ctrl combinations
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('z') => {
                app.undo();
                return;
            }
            KeyCode::Char('y') => {
                app.redo();
                return;
            }
            KeyCode::Char('s') => {
                // Save
                if !app.save_project() {
                    // No path set — prompt for name
                    app.text_input = app
                        .project_name
                        .clone()
                        .unwrap_or_else(|| "untitled".to_string());
                    app.mode = AppMode::SaveAs;
                }
                return;
            }
            KeyCode::Char('o') => {
                // Open file dialog
                app.open_file_dialog();
                return;
            }
            KeyCode::Char('n') => {
                // New canvas dialog
                app.new_canvas_width = app.canvas.width;
                app.new_canvas_height = app.canvas.height;
                app.new_canvas_cursor = 0;
                app.new_canvas_input = app.canvas.width.to_string();
                app.mode = AppMode::NewCanvas;
                return;
            }
            KeyCode::Char('r') => {
                // Resize canvas dialog
                app.new_canvas_width = app.canvas.width;
                app.new_canvas_height = app.canvas.height;
                app.new_canvas_cursor = 0;
                app.new_canvas_input = app.canvas.width.to_string();
                app.mode = AppMode::ResizeCanvas;
                return;
            }
            KeyCode::Char('t') => {
                app.cycle_theme();
                return;
            }
            KeyCode::Char('e') => {
                // Export dialog
                app.export_format = 0;
                app.export_dest = 0;
                app.export_cursor = 0;
                app.export_color_format = 0;
                app.mode = AppMode::ExportDialog;
                return;
            }
            KeyCode::Char('i') => {
                // Import image dialog
                open_import_dialog(app);
                return;
            }
            KeyCode::Char('c') => {
                if app.dirty {
                    app.mode = AppMode::Quitting;
                    app.set_status("Unsaved changes. Quit? (y/n)");
                } else {
                    app.running = false;
                }
                return;
            }
            _ => return,
        }
    }

    match key.code {
        // Tool selection
        KeyCode::Char('p') | KeyCode::Char('P') => {
            app.active_tool = ToolKind::Pencil;
            app.cancel_tool();
        }
        KeyCode::Char('e') | KeyCode::Char('E') => {
            app.active_tool = ToolKind::Eraser;
            app.cancel_tool();
        }
        KeyCode::Char('l') | KeyCode::Char('L') => {
            app.active_tool = ToolKind::Line;
            app.cancel_tool();
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            app.active_tool = ToolKind::Rectangle;
            app.cancel_tool();
        }
        KeyCode::Char('f') | KeyCode::Char('F') => {
            app.active_tool = ToolKind::Fill;
            app.cancel_tool();
        }
        KeyCode::Char('i') | KeyCode::Char('I') => {
            app.active_tool = ToolKind::Eyedropper;
            app.cancel_tool();
        }

        // Symmetry
        KeyCode::Char('h') | KeyCode::Char('H') => {
            app.symmetry = app.symmetry.toggle_horizontal();
            app.set_status(&format!("Symmetry: {}", app.symmetry.label()));
        }
        KeyCode::Char('v') | KeyCode::Char('V') => {
            app.symmetry = app.symmetry.toggle_vertical();
            app.set_status(&format!("Symmetry: {}", app.symmetry.label()));
        }

        // Zoom cycle
        KeyCode::Char('z') | KeyCode::Char('Z') => {
            app.cycle_zoom();
        }

        // Quick color pick: 1-9 → curated palette slots 0-8, 0 → slot 9
        KeyCode::Char(c @ '1'..='9') => {
            let n = (c as u8 - b'1') as usize;
            app.quick_pick_color(n);
        }
        KeyCode::Char('0') => {
            app.quick_pick_color(9);
        }

        // Palette navigation (uses palette_layout)
        KeyCode::Up => {
            if app.palette_cursor > 0 {
                app.palette_cursor -= 1;
                if let Some(PaletteItem::Color(color)) = app.palette_layout.get(app.palette_cursor) {
                    app.color = *color;
                }
                app.ensure_palette_cursor_visible(15);
            }
        }
        KeyCode::Down => {
            if app.palette_cursor + 1 < app.palette_layout.len() {
                app.palette_cursor += 1;
                if let Some(PaletteItem::Color(color)) = app.palette_layout.get(app.palette_cursor) {
                    app.color = *color;
                }
                app.ensure_palette_cursor_visible(15);
            }
        }
        KeyCode::Left => {
            if app.palette_cursor >= 6 {
                app.palette_cursor -= 6;
                if let Some(PaletteItem::Color(color)) = app.palette_layout.get(app.palette_cursor) {
                    app.color = *color;
                }
                app.ensure_palette_cursor_visible(15);
            }
        }
        KeyCode::Right => {
            if app.palette_cursor + 6 < app.palette_layout.len() {
                app.palette_cursor += 6;
                if let Some(PaletteItem::Color(color)) = app.palette_layout.get(app.palette_cursor) {
                    app.color = *color;
                }
                app.ensure_palette_cursor_visible(15);
            }
        }
        // Enter on palette: toggle section header or select color
        KeyCode::Enter => {
            if let Some(item) = app.palette_layout.get(app.palette_cursor).copied() {
                match item {
                    PaletteItem::SectionHeader(section) => {
                        match section {
                            PaletteSection::Recent => {
                                app.palette_sections.recent_expanded = !app.palette_sections.recent_expanded;
                            }
                            PaletteSection::Standard => {
                                app.palette_sections.standard_expanded = !app.palette_sections.standard_expanded;
                            }
                            PaletteSection::HueGroups => {
                                app.palette_sections.hue_expanded = !app.palette_sections.hue_expanded;
                            }
                            PaletteSection::Grayscale => {
                                app.palette_sections.grayscale_expanded = !app.palette_sections.grayscale_expanded;
                            }
                        }
                        app.rebuild_palette_layout();
                        // Clamp cursor if layout shrank
                        if app.palette_cursor >= app.palette_layout.len() {
                            app.palette_cursor = app.palette_layout.len().saturating_sub(1);
                        }
                    }
                    PaletteItem::Color(color) => {
                        app.color = color;
                    }
                }
            }
        }

        // WASD canvas navigation
        KeyCode::Char('w') | KeyCode::Char('W') => {
            app.canvas_cursor.1 = app.canvas_cursor.1.saturating_sub(1);
            app.canvas_cursor_active = true;
            let (cx, cy) = app.canvas_cursor;
            app.ensure_cursor_in_viewport(cx, cy, app.viewport_w, app.viewport_h);
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            app.canvas_cursor.0 = (app.canvas_cursor.0 + 1).min(app.canvas.width.saturating_sub(1));
            app.canvas_cursor_active = true;
            let (cx, cy) = app.canvas_cursor;
            app.ensure_cursor_in_viewport(cx, cy, app.viewport_w, app.viewport_h);
        }
        KeyCode::Char(' ') => {
            if app.canvas_cursor_active {
                let (x, y) = app.canvas_cursor;
                if matches!(app.active_tool, ToolKind::Pencil | ToolKind::Eraser) {
                    app.begin_stroke();
                }
                app.apply_tool(x, y);
                if matches!(app.active_tool, ToolKind::Pencil | ToolKind::Eraser) {
                    app.end_stroke();
                }
            }
        }

        // S key: canvas down if active, otherwise HSL sliders
        KeyCode::Char('s') | KeyCode::Char('S') => {
            if app.canvas_cursor_active {
                app.canvas_cursor.1 = (app.canvas_cursor.1 + 1).min(app.canvas.height.saturating_sub(1));
                let (cx, cy) = app.canvas_cursor;
                app.ensure_cursor_in_viewport(cx, cy, app.viewport_w, app.viewport_h);
            } else {
                let (h, s, l) = crate::palette::rgb_to_hsl(app.color.r, app.color.g, app.color.b);
                app.slider_h = h;
                app.slider_s = s;
                app.slider_l = l;
                app.slider_active = 0;
                app.mode = AppMode::ColorSliders;
            }
        }

        // A key: canvas left if active, otherwise add to palette
        KeyCode::Char('a') | KeyCode::Char('A') => {
            if app.canvas_cursor_active {
                app.canvas_cursor.0 = app.canvas_cursor.0.saturating_sub(1);
                let (cx, cy) = app.canvas_cursor;
                app.ensure_cursor_in_viewport(cx, cy, app.viewport_w, app.viewport_h);
            } else {
                app.add_color_to_custom_palette();
            }
        }

        // Custom palette dialog
        KeyCode::Char('c') | KeyCode::Char('C') => {
            app.open_palette_dialog();
        }

        // Cycle block character type
        KeyCode::Char('b') => {
            app.cycle_block();
        }
        KeyCode::Char('B') => {
            app.open_block_picker();
        }

        // Shade cycle (G key)
        KeyCode::Char('g') | KeyCode::Char('G') => {
            app.cycle_shade();
        }

        // Toggle filled/outline rectangle
        KeyCode::Char('t') | KeyCode::Char('T') => {
            app.filled_rect = !app.filled_rect;
            app.set_status(if app.filled_rect { "Rect: Filled" } else { "Rect: Outline" });
        }

        // Hex color input dialog
        KeyCode::Char('x') | KeyCode::Char('X') => {
            app.text_input = String::new();
            app.mode = AppMode::HexColorInput;
        }

        // Cancel multi-click tool / deactivate canvas cursor
        KeyCode::Esc => {
            if app.canvas_cursor_active {
                app.canvas_cursor_active = false;
                app.set_status("Canvas cursor off");
            } else {
                app.cancel_tool();
                app.set_status("Cancelled");
            }
        }

        // Help
        KeyCode::Char('?') => {
            app.mode = AppMode::Help;
        }

        // Quit
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            if app.dirty {
                app.mode = AppMode::Quitting;
                app.set_status("Unsaved changes. Quit? (y/n)");
            } else {
                app.running = false;
            }
        }

        _ => {}
    }
}

fn handle_file_dialog(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Up => {
            if app.file_dialog_selected > 0 {
                app.file_dialog_selected -= 1;
            }
        }
        KeyCode::Down => {
            if app.file_dialog_selected + 1 < app.file_dialog_files.len() {
                app.file_dialog_selected += 1;
            }
        }
        KeyCode::Enter => {
            if let Some(filename) = app.file_dialog_files.get(app.file_dialog_selected).cloned() {
                app.mode = AppMode::Normal;
                app.load_project(&filename);
            }
        }
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
        }
        _ => {}
    }
}

fn handle_export_dialog(app: &mut App, code: KeyCode) {
    // Row count: 0=format, 1=dest; if ANSI: 0=format, 1=color_format, 2=dest
    let max_row = if app.export_format == 1 { 2 } else { 1 };

    match code {
        KeyCode::Up => {
            if app.export_cursor > 0 {
                app.export_cursor -= 1;
            }
        }
        KeyCode::Down => {
            if app.export_cursor < max_row {
                app.export_cursor += 1;
            }
        }
        KeyCode::Left | KeyCode::Right => {
            if app.export_cursor == 0 {
                // Toggle format: PlainText <-> ANSI
                app.export_format = 1 - app.export_format;
                // Clamp cursor when switching from ANSI to plain text
                if app.export_format == 0 && app.export_cursor > 1 {
                    app.export_cursor = 1;
                }
            } else if app.export_format == 1 && app.export_cursor == 1 {
                // Color format row (only when ANSI): cycle 0/1/2
                if code == KeyCode::Right {
                    app.export_color_format = (app.export_color_format + 1) % 3;
                } else {
                    app.export_color_format = (app.export_color_format + 2) % 3;
                }
            } else {
                // Dest row
                app.export_dest = 1 - app.export_dest;
            }
        }
        KeyCode::Enter => {
            app.do_export();
        }
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
        }
        _ => {}
    }
}

enum TextInputPurpose {
    SaveAs,
    ExportFile,
    PaletteName,
    PaletteRename,
    PaletteExport,
}

fn handle_text_input(app: &mut App, key: KeyEvent, purpose: TextInputPurpose) {
    match key.code {
        KeyCode::Enter => {
            let input = app.text_input.clone();
            if input.trim().is_empty() {
                app.set_status("Name cannot be empty");
                return;
            }
            match purpose {
                TextInputPurpose::SaveAs => {
                    app.mode = AppMode::Normal;
                    app.save_as(input.trim());
                }
                TextInputPurpose::ExportFile => {
                    app.export_to_file(input.trim());
                }
                TextInputPurpose::PaletteName => {
                    app.create_custom_palette(input.trim());
                }
                TextInputPurpose::PaletteRename => {
                    app.rename_selected_palette(input.trim());
                }
                TextInputPurpose::PaletteExport => {
                    app.export_selected_palette(input.trim());
                }
            }
        }
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
        }
        KeyCode::Backspace => {
            app.text_input.pop();
        }
        KeyCode::Char(c) => {
            if app.text_input.len() < 64 {
                app.text_input.push(c);
            }
        }
        _ => {}
    }
}

fn handle_color_sliders(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Up => {
            if app.slider_active > 0 {
                app.slider_active -= 1;
            }
        }
        KeyCode::Down => {
            if app.slider_active < 2 {
                app.slider_active += 1;
            }
        }
        KeyCode::Left => {
            match app.slider_active {
                0 => app.slider_h = app.slider_h.saturating_sub(5),
                1 => app.slider_s = app.slider_s.saturating_sub(5),
                _ => app.slider_l = app.slider_l.saturating_sub(5),
            }
        }
        KeyCode::Right => {
            match app.slider_active {
                0 => app.slider_h = (app.slider_h + 5).min(359),
                1 => app.slider_s = (app.slider_s + 5).min(100),
                _ => app.slider_l = (app.slider_l + 5).min(100),
            }
        }
        KeyCode::Enter => {
            let (r, g, b) = crate::palette::hsl_to_rgb(app.slider_h, app.slider_s, app.slider_l);
            let color = crate::palette::nearest_color(r, g, b);
            app.color = color;
            app.mode = AppMode::Normal;
            app.set_status(&format!("Color: {}", color.name()));
        }
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
        }
        _ => {}
    }
}

fn handle_palette_dialog(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Up => {
            if app.palette_dialog_selected > 0 {
                app.palette_dialog_selected -= 1;
            }
        }
        KeyCode::Down => {
            if app.palette_dialog_selected + 1 < app.palette_dialog_files.len() {
                app.palette_dialog_selected += 1;
            }
        }
        KeyCode::Enter => {
            app.load_selected_palette();
        }
        KeyCode::Char('n') | KeyCode::Char('N') => {
            app.text_input = String::new();
            app.mode = AppMode::PaletteNameInput;
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            app.delete_selected_palette();
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            if !app.palette_dialog_files.is_empty() {
                // Pre-fill with current name (without .palette extension)
                if let Some(filename) = app.palette_dialog_files.get(app.palette_dialog_selected) {
                    app.text_input = filename.trim_end_matches(".palette").to_string();
                }
                app.mode = AppMode::PaletteRename;
            }
        }
        KeyCode::Char('u') | KeyCode::Char('U') => {
            app.duplicate_selected_palette();
        }
        KeyCode::Char('x') | KeyCode::Char('X') => {
            if !app.palette_dialog_files.is_empty() {
                if let Some(filename) = app.palette_dialog_files.get(app.palette_dialog_selected) {
                    app.text_input = filename.clone();
                }
                app.mode = AppMode::PaletteExport;
            }
        }
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
        }
        _ => {}
    }
}

/// Parse the input buffer into a dimension value, falling back to the given default.
fn parse_canvas_input(input: &str, default: usize) -> usize {
    if input.is_empty() {
        default
    } else {
        input.parse::<usize>().unwrap_or(default)
    }
}

/// Sync the text buffer to the stored width/height, then load the other field into the buffer.
fn switch_canvas_field(app: &mut App) {
    use crate::canvas::{MIN_DIMENSION, MAX_DIMENSION};
    // Store current buffer into active field
    let val = parse_canvas_input(&app.new_canvas_input, if app.new_canvas_cursor == 0 { app.new_canvas_width } else { app.new_canvas_height });
    let clamped = val.clamp(MIN_DIMENSION, MAX_DIMENSION);
    if app.new_canvas_cursor == 0 {
        app.new_canvas_width = clamped;
    } else {
        app.new_canvas_height = clamped;
    }
    // Switch cursor
    app.new_canvas_cursor = 1 - app.new_canvas_cursor;
    // Load other field into buffer
    let other_val = if app.new_canvas_cursor == 0 { app.new_canvas_width } else { app.new_canvas_height };
    app.new_canvas_input = other_val.to_string();
}

fn handle_new_canvas(app: &mut App, code: KeyCode) {
    use crate::canvas::{MIN_DIMENSION, MAX_DIMENSION};

    match code {
        KeyCode::Up | KeyCode::Down | KeyCode::Tab => {
            switch_canvas_field(app);
        }
        KeyCode::Left => {
            // ±1 decrement
            let val = parse_canvas_input(&app.new_canvas_input, if app.new_canvas_cursor == 0 { app.new_canvas_width } else { app.new_canvas_height });
            let new_val = val.saturating_sub(1).max(MIN_DIMENSION);
            if app.new_canvas_cursor == 0 {
                app.new_canvas_width = new_val;
            } else {
                app.new_canvas_height = new_val;
            }
            app.new_canvas_input = new_val.to_string();
        }
        KeyCode::Right => {
            // ±1 increment
            let val = parse_canvas_input(&app.new_canvas_input, if app.new_canvas_cursor == 0 { app.new_canvas_width } else { app.new_canvas_height });
            let new_val = (val + 1).min(MAX_DIMENSION);
            if app.new_canvas_cursor == 0 {
                app.new_canvas_width = new_val;
            } else {
                app.new_canvas_height = new_val;
            }
            app.new_canvas_input = new_val.to_string();
        }
        KeyCode::Char(c) if c.is_ascii_digit() => {
            if app.new_canvas_input.len() < 3 {
                app.new_canvas_input.push(c);
            }
        }
        KeyCode::Backspace => {
            app.new_canvas_input.pop();
        }
        KeyCode::Enter => {
            // Parse active field; empty input falls back to current canvas size
            let default_dim = if app.new_canvas_cursor == 0 { app.canvas.width } else { app.canvas.height };
            let buf_val = parse_canvas_input(&app.new_canvas_input, default_dim);
            if app.new_canvas_cursor == 0 {
                app.new_canvas_width = buf_val;
            } else {
                app.new_canvas_height = buf_val;
            }
            let w = app.new_canvas_width.clamp(MIN_DIMENSION, MAX_DIMENSION);
            let h = app.new_canvas_height.clamp(MIN_DIMENSION, MAX_DIMENSION);
            let clamped = w != app.new_canvas_width || h != app.new_canvas_height;
            app.new_canvas_width = w;
            app.new_canvas_height = h;
            app.canvas = Canvas::new_with_size(w, h);
            app.history = History::new();
            app.dirty = false;
            app.project_name = None;
            app.project_path = None;
            app.cursor = None;
            app.canvas_cursor = (0, 0);
            app.canvas_cursor_active = false;
            app.viewport_x = 0;
            app.viewport_y = 0;
            app.tool_state = ToolState::Idle;
            app.mode = AppMode::Normal;
            if clamped {
                app.set_status(&format!("New canvas {}x{} (clamped to {}-{})", w, h, MIN_DIMENSION, MAX_DIMENSION));
            } else {
                app.set_status(&format!("New canvas {}x{}", w, h));
            }
        }
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
        }
        _ => {}
    }
}

fn handle_resize_canvas(app: &mut App, code: KeyCode) {
    use crate::canvas::{MIN_DIMENSION, MAX_DIMENSION};

    match code {
        KeyCode::Up | KeyCode::Down | KeyCode::Tab => {
            switch_canvas_field(app);
        }
        KeyCode::Left => {
            let val = parse_canvas_input(&app.new_canvas_input, if app.new_canvas_cursor == 0 { app.new_canvas_width } else { app.new_canvas_height });
            let new_val = val.saturating_sub(1).max(MIN_DIMENSION);
            if app.new_canvas_cursor == 0 {
                app.new_canvas_width = new_val;
            } else {
                app.new_canvas_height = new_val;
            }
            app.new_canvas_input = new_val.to_string();
        }
        KeyCode::Right => {
            let val = parse_canvas_input(&app.new_canvas_input, if app.new_canvas_cursor == 0 { app.new_canvas_width } else { app.new_canvas_height });
            let new_val = (val + 1).min(MAX_DIMENSION);
            if app.new_canvas_cursor == 0 {
                app.new_canvas_width = new_val;
            } else {
                app.new_canvas_height = new_val;
            }
            app.new_canvas_input = new_val.to_string();
        }
        KeyCode::Char(c) if c.is_ascii_digit() => {
            if app.new_canvas_input.len() < 3 {
                app.new_canvas_input.push(c);
            }
        }
        KeyCode::Backspace => {
            app.new_canvas_input.pop();
        }
        KeyCode::Enter => {
            // Parse active field
            let default_dim = if app.new_canvas_cursor == 0 { app.canvas.width } else { app.canvas.height };
            let buf_val = parse_canvas_input(&app.new_canvas_input, default_dim);
            if app.new_canvas_cursor == 0 {
                app.new_canvas_width = buf_val;
            } else {
                app.new_canvas_height = buf_val;
            }
            let w = app.new_canvas_width.clamp(MIN_DIMENSION, MAX_DIMENSION);
            let h = app.new_canvas_height.clamp(MIN_DIMENSION, MAX_DIMENSION);
            app.new_canvas_width = w;
            app.new_canvas_height = h;

            // Same size → no-op
            if w == app.canvas.width && h == app.canvas.height {
                app.mode = AppMode::Normal;
                app.set_status("Same size — no resize needed");
                return;
            }

            // Shrinking → show crop warning
            if w < app.canvas.width || h < app.canvas.height {
                app.mode = AppMode::ResizeCropConfirm;
                return;
            }

            // Enlarging → apply immediately
            do_resize(app, w, h);
        }
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
        }
        _ => {}
    }
}

fn handle_resize_crop_confirm(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
            let w = app.new_canvas_width;
            let h = app.new_canvas_height;
            do_resize(app, w, h);
        }
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            app.set_status("Resize cancelled");
        }
        _ => {}
    }
}

/// Execute the resize with CanvasSnapshot for undo.
fn do_resize(app: &mut App, w: usize, h: usize) {
    // Step 1: capture old snapshot
    let old_cells = app.canvas.cells();
    let old_w = app.canvas.width;
    let old_h = app.canvas.height;

    // Step 2: resize
    app.canvas.resize(w, h);

    // Step 3: capture new snapshot
    let new_cells = app.canvas.cells();
    let new_w = app.canvas.width;
    let new_h = app.canvas.height;

    // Step 4: push to history
    app.history.commit(Action::CanvasSnapshot {
        old_cells, old_w, old_h,
        new_cells, new_w, new_h,
    });

    // Step 5: reset viewport
    app.viewport_x = 0;
    app.viewport_y = 0;

    app.dirty = true;
    app.mode = AppMode::Normal;
    app.set_status(&format!("Resized to {}x{}", new_w, new_h));
}

fn handle_hex_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            match crate::cell::parse_hex_color(&app.text_input) {
                Some(rgb) => {
                    let matched = crate::palette::nearest_color(rgb.r, rgb.g, rgb.b);
                    app.color = matched;
                    app.mode = AppMode::Normal;
                    app.set_status(&format!("Color: {} → {}", rgb.name(), matched.name()));
                }
                None => {
                    app.set_status("Invalid hex (use #RRGGBB)");
                }
            }
        }
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
        }
        KeyCode::Backspace => {
            app.text_input.pop();
        }
        KeyCode::Char(c) => {
            if app.text_input.len() < 7 {
                app.text_input.push(c);
            }
        }
        _ => {}
    }
}

fn handle_block_picker(app: &mut App, key: KeyEvent) {
    use crate::cell::blocks;
    let sizes = blocks::CATEGORY_SIZES;
    let num_rows = sizes.len();

    match key.code {
        KeyCode::Left => {
            if app.block_picker_col > 0 {
                app.block_picker_col -= 1;
            }
        }
        KeyCode::Right => {
            let max_col = sizes[app.block_picker_row].saturating_sub(1);
            if app.block_picker_col < max_col {
                app.block_picker_col += 1;
            }
        }
        KeyCode::Up => {
            if app.block_picker_row > 0 {
                app.block_picker_row -= 1;
                // Clamp column to new row's width
                let max_col = sizes[app.block_picker_row].saturating_sub(1);
                if app.block_picker_col > max_col {
                    app.block_picker_col = max_col;
                }
            }
        }
        KeyCode::Down => {
            if app.block_picker_row < num_rows - 1 {
                app.block_picker_row += 1;
                // Clamp column to new row's width
                let max_col = sizes[app.block_picker_row].saturating_sub(1);
                if app.block_picker_col > max_col {
                    app.block_picker_col = max_col;
                }
            }
        }
        KeyCode::Enter => {
            // Convert (row, col) to flat index into blocks::ALL
            let offset: usize = sizes[..app.block_picker_row].iter().sum();
            let idx = offset + app.block_picker_col;
            if idx < blocks::ALL.len() {
                app.active_block = blocks::ALL[idx];
                app.set_status(&format!("Block: {}", app.active_block));
            }
            app.mode = AppMode::Normal;
        }
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
        }
        _ => {}
    }
}

fn handle_mouse(app: &mut App, mouse: MouseEvent, canvas_area: &CanvasArea) {
    let zoom = app.zoom;
    let vp_x = app.viewport_x;
    let vp_y = app.viewport_y;
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if let Some((x, y)) = canvas_area.screen_to_canvas(mouse.column, mouse.row, zoom, vp_x, vp_y) {
                app.cursor = Some((x, y));
                app.canvas_cursor = (x, y);
                app.canvas_cursor_active = false;
                // Start stroke for continuous tools
                if matches!(app.active_tool, ToolKind::Pencil | ToolKind::Eraser) {
                    app.begin_stroke();
                }
                app.apply_tool(x, y);
            }
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if let Some((x, y)) = canvas_area.screen_to_canvas(mouse.column, mouse.row, zoom, vp_x, vp_y) {
                app.cursor = Some((x, y));
                if matches!(app.active_tool, ToolKind::Pencil | ToolKind::Eraser) {
                    app.apply_tool(x, y);
                }
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            if app.history.is_stroke_active() {
                app.end_stroke();
            }
        }
        MouseEventKind::Down(MouseButton::Right) => {
            // Quick eyedropper
            if let Some((x, y)) = canvas_area.screen_to_canvas(mouse.column, mouse.row, zoom, vp_x, vp_y) {
                if let Some((picked_fg, _bg, ch)) = crate::tools::eyedropper(&app.canvas, x, y) {
                    if let Some(picked) = picked_fg {
                        app.color = picked;
                        app.set_status(&format!("Picked: {} {}", picked.name(), ch));
                    }
                    if ch != ' ' {
                        app.active_block = ch;
                    }
                }
            }
        }
        MouseEventKind::Moved => {
            if let Some((x, y)) = canvas_area.screen_to_canvas(mouse.column, mouse.row, zoom, vp_x, vp_y) {
                app.cursor = Some((x, y));
                app.canvas_cursor_active = false;
            } else {
                app.cursor = None;
            }
        }
        _ => {}
    }
}

/// Image file extensions accepted by the import browser.
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "bmp"];

/// Check if a filename has an image extension.
fn is_image_file(name: &str) -> bool {
    let lower = name.to_lowercase();
    IMAGE_EXTENSIONS.iter().any(|ext| lower.ends_with(&format!(".{}", ext)))
}

/// List image files and directories in a given directory.
fn list_import_entries(dir: &std::path::Path) -> Vec<String> {
    let mut entries = Vec::new();

    // Add parent directory navigation (unless at root)
    if dir.parent().is_some() {
        entries.push("..".to_string());
    }

    if let Ok(read_dir) = std::fs::read_dir(dir) {
        let mut dirs = Vec::new();
        let mut files = Vec::new();
        for entry in read_dir.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                // Skip hidden files
                if name.starts_with('.') {
                    continue;
                }
                if path.is_dir() {
                    dirs.push(format!("{}/", name));
                } else if is_image_file(name) {
                    files.push(name.to_string());
                }
            }
        }
        dirs.sort();
        files.sort();
        entries.extend(dirs);
        entries.extend(files);
    }

    entries
}

/// Open the import file browser dialog.
fn open_import_dialog(app: &mut App) {
    let entries = list_import_entries(&app.import_dir);
    app.file_dialog_files = entries;
    app.file_dialog_selected = 0;
    if app.file_dialog_files.is_empty() {
        app.set_status("No image files found");
    } else {
        app.mode = AppMode::ImportBrowse;
    }
}

fn handle_import_browse(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Up => {
            if app.file_dialog_selected > 0 {
                app.file_dialog_selected -= 1;
            }
        }
        KeyCode::Down => {
            if app.file_dialog_selected + 1 < app.file_dialog_files.len() {
                app.file_dialog_selected += 1;
            }
        }
        KeyCode::Enter => {
            if let Some(entry) = app.file_dialog_files.get(app.file_dialog_selected).cloned() {
                if entry == ".." {
                    // Navigate to parent directory
                    if let Some(parent) = app.import_dir.parent() {
                        app.import_dir = parent.to_path_buf();
                    }
                    app.file_dialog_files = list_import_entries(&app.import_dir);
                    app.file_dialog_selected = 0;
                } else if entry.ends_with('/') {
                    // Navigate into directory
                    let dir_name = &entry[..entry.len() - 1];
                    app.import_dir = app.import_dir.join(dir_name);
                    app.file_dialog_files = list_import_entries(&app.import_dir);
                    app.file_dialog_selected = 0;
                } else {
                    // Image file selected — store path and go to options
                    let full_path = app.import_dir.join(&entry);
                    app.import_path = Some(full_path);
                    app.import_options_cursor = 0;
                    app.mode = AppMode::ImportOptions;
                }
            }
        }
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
        }
        _ => {}
    }
}

fn handle_import_options(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Up => {
            if app.import_options_cursor > 0 {
                app.import_options_cursor -= 1;
            }
        }
        KeyCode::Down => {
            if app.import_options_cursor < 2 {
                app.import_options_cursor += 1;
            }
        }
        KeyCode::Left | KeyCode::Right => {
            match app.import_options_cursor {
                0 => app.import_fit = 1 - app.import_fit,
                1 => app.import_color = 1 - app.import_color,
                2 => app.import_charset = 1 - app.import_charset,
                _ => {}
            }
        }
        KeyCode::Enter => {
            do_import(app);
        }
        KeyCode::Esc => {
            // Return to browse
            app.mode = AppMode::ImportBrowse;
        }
        _ => {}
    }
}

fn do_import(app: &mut App) {
    use crate::import::{self, FitMode, ImportCharSet, ImportColorMode, ImportOptions as ImportOpts};

    let path = match &app.import_path {
        Some(p) => p.clone(),
        None => {
            app.set_status("No file selected");
            app.mode = AppMode::Normal;
            return;
        }
    };

    let fit_mode = if app.import_fit == 0 {
        FitMode::FitToCanvas
    } else {
        FitMode::CustomSize(app.canvas.width, app.canvas.height)
    };

    let color_mode = if app.import_color == 0 {
        ImportColorMode::Color256
    } else {
        ImportColorMode::Color16
    };

    let char_set = if app.import_charset == 0 {
        ImportCharSet::FullBlocks
    } else {
        ImportCharSet::HalfBlocks
    };

    let opts = ImportOpts {
        fit_mode,
        color_mode,
        char_set,
    };

    let target_w = app.canvas.width;
    let target_h = app.canvas.height;

    match import::import_image(&path, target_w, target_h, &opts) {
        Ok(cells) => {
            // Snapshot for undo
            let old_cells = app.canvas.cells();
            let old_w = app.canvas.width;
            let old_h = app.canvas.height;

            // Apply imported cells to canvas (clamped to canvas bounds)
            for (y, row) in cells.iter().take(app.canvas.height).enumerate() {
                for (x, cell) in row.iter().take(app.canvas.width).enumerate() {
                    app.canvas.set(x, y, *cell);
                }
            }

            let new_cells = app.canvas.cells();
            let new_w = app.canvas.width;
            let new_h = app.canvas.height;

            app.history.commit(Action::CanvasSnapshot {
                old_cells, old_w, old_h,
                new_cells, new_w, new_h,
            });

            app.dirty = true;
            app.mode = AppMode::Normal;
            app.viewport_x = 0;
            app.viewport_y = 0;

            // Check if GIF via extension
            let is_gif = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("gif"))
                .unwrap_or(false);
            if is_gif {
                app.set_status("Imported (GIF: first frame only)");
            } else {
                app.set_status("Image imported");
            }
        }
        Err(e) => {
            app.set_status(&format!("Import failed: {}", e));
            app.mode = AppMode::Normal;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area() -> CanvasArea {
        CanvasArea { left: 10, top: 5, width: 64, height: 32, viewport_w: 64, viewport_h: 32 }
    }

    #[test]
    fn test_screen_to_canvas_zoom_1() {
        let a = area();
        assert_eq!(a.screen_to_canvas(10, 5, 1, 0, 0), Some((0, 0)));
        assert_eq!(a.screen_to_canvas(14, 8, 1, 0, 0), Some((4, 3)));
    }

    #[test]
    fn test_screen_to_canvas_zoom_2() {
        let a = area();
        assert_eq!(a.screen_to_canvas(10, 5, 2, 0, 0), Some((0, 0)));
        assert_eq!(a.screen_to_canvas(14, 8, 2, 0, 0), Some((2, 3)));
    }

    #[test]
    fn test_screen_to_canvas_zoom_4() {
        let a = area();
        assert_eq!(a.screen_to_canvas(10, 5, 4, 0, 0), Some((0, 0)));
        assert_eq!(a.screen_to_canvas(14, 9, 4, 0, 0), Some((1, 2)));
    }

    #[test]
    fn test_screen_to_canvas_outside() {
        let a = area();
        assert_eq!(a.screen_to_canvas(5, 5, 1, 0, 0), None);
        assert_eq!(a.screen_to_canvas(10, 3, 1, 0, 0), None);
        assert_eq!(a.screen_to_canvas(80, 5, 1, 0, 0), None);
    }

    #[test]
    fn test_screen_to_canvas_with_viewport_offset() {
        let a = area();
        // With viewport at (10, 5), the first screen cell maps to canvas (10, 5)
        assert_eq!(a.screen_to_canvas(10, 5, 1, 10, 5), Some((10, 5)));
        assert_eq!(a.screen_to_canvas(14, 8, 1, 10, 5), Some((14, 8)));
    }

    // --- Cycle 16: NewCanvas free-text input tests ---

    #[test]
    fn test_new_canvas_digit_input() {
        let mut app = App::new();
        app.mode = AppMode::NewCanvas;
        app.new_canvas_cursor = 0;
        app.new_canvas_input = String::new();

        handle_new_canvas(&mut app, KeyCode::Char('5'));
        handle_new_canvas(&mut app, KeyCode::Char('0'));
        assert_eq!(app.new_canvas_input, "50");
    }

    #[test]
    fn test_new_canvas_backspace() {
        let mut app = App::new();
        app.mode = AppMode::NewCanvas;
        app.new_canvas_cursor = 0;
        app.new_canvas_input = "50".to_string();

        handle_new_canvas(&mut app, KeyCode::Backspace);
        assert_eq!(app.new_canvas_input, "5");
        handle_new_canvas(&mut app, KeyCode::Backspace);
        assert_eq!(app.new_canvas_input, "");
    }

    #[test]
    fn test_new_canvas_clamp_min() {
        let mut app = App::new();
        app.mode = AppMode::NewCanvas;
        app.new_canvas_cursor = 0;
        app.new_canvas_input = "3".to_string();

        handle_new_canvas(&mut app, KeyCode::Enter);
        // Should create canvas with width clamped to MIN_DIMENSION (8)
        assert_eq!(app.canvas.width, crate::canvas::MIN_DIMENSION);
        assert_eq!(app.mode, AppMode::Normal);
    }

    #[test]
    fn test_new_canvas_clamp_max() {
        let mut app = App::new();
        app.mode = AppMode::NewCanvas;
        app.new_canvas_cursor = 0;
        app.new_canvas_input = "200".to_string();
        app.new_canvas_height = 32;

        handle_new_canvas(&mut app, KeyCode::Enter);
        // Should create canvas with width clamped to MAX_DIMENSION (128)
        assert_eq!(app.canvas.width, crate::canvas::MAX_DIMENSION);
        assert_eq!(app.mode, AppMode::Normal);
    }

    #[test]
    fn test_new_canvas_arrow_increment() {
        let mut app = App::new();
        app.mode = AppMode::NewCanvas;
        app.new_canvas_cursor = 0;
        app.new_canvas_width = 48;
        app.new_canvas_input = "48".to_string();

        handle_new_canvas(&mut app, KeyCode::Right);
        assert_eq!(app.new_canvas_input, "49");
        assert_eq!(app.new_canvas_width, 49);

        handle_new_canvas(&mut app, KeyCode::Left);
        assert_eq!(app.new_canvas_input, "48");
        assert_eq!(app.new_canvas_width, 48);
    }

    #[test]
    fn test_new_canvas_tab_switch() {
        let mut app = App::new();
        app.mode = AppMode::NewCanvas;
        app.new_canvas_cursor = 0;
        app.new_canvas_width = 48;
        app.new_canvas_height = 32;
        app.new_canvas_input = "48".to_string();

        handle_new_canvas(&mut app, KeyCode::Tab);
        assert_eq!(app.new_canvas_cursor, 1);
        assert_eq!(app.new_canvas_input, "32"); // loaded height
        assert_eq!(app.new_canvas_width, 48); // width stored
    }

    #[test]
    fn test_new_canvas_empty_input() {
        let mut app = App::new();
        app.mode = AppMode::NewCanvas;
        app.new_canvas_cursor = 0;
        app.new_canvas_width = 48;
        app.new_canvas_height = 32;
        app.new_canvas_input = String::new(); // empty

        handle_new_canvas(&mut app, KeyCode::Enter);
        // Empty input uses current width (48)
        assert_eq!(app.canvas.width, 48);
        assert_eq!(app.canvas.height, 32);
    }

    // --- Cycle 16: Resize Canvas tests ---

    #[test]
    fn test_resize_larger_preserves_content() {
        let mut app = App::new();
        // Place a cell
        let cell = crate::cell::Cell {
            ch: crate::cell::blocks::FULL,
            fg: Some(crate::cell::Rgb { r: 205, g: 0, b: 0 }),
            bg: None,
        };
        app.canvas.set(5, 5, cell);

        // Resize larger via do_resize
        app.new_canvas_width = 64;
        app.new_canvas_height = 48;
        do_resize(&mut app, 64, 48);

        assert_eq!(app.canvas.width, 64);
        assert_eq!(app.canvas.height, 48);
        assert_eq!(app.canvas.get(5, 5), Some(cell));
        assert_eq!(app.mode, AppMode::Normal);
    }

    #[test]
    fn test_resize_smaller_warns() {
        let mut app = App::new();
        app.mode = AppMode::ResizeCanvas;
        app.new_canvas_width = 16;
        app.new_canvas_height = 16;
        app.new_canvas_input = "16".to_string();
        app.new_canvas_cursor = 0;

        // Enter on a shrink should go to crop confirm
        handle_resize_canvas(&mut app, KeyCode::Enter);
        assert_eq!(app.mode, AppMode::ResizeCropConfirm);
    }

    #[test]
    fn test_resize_undo_restore() {
        let mut app = App::new();
        let cell = crate::cell::Cell {
            ch: crate::cell::blocks::FULL,
            fg: Some(crate::cell::Rgb { r: 205, g: 0, b: 0 }),
            bg: None,
        };
        app.canvas.set(10, 10, cell);
        let orig_w = app.canvas.width;
        let orig_h = app.canvas.height;

        // Resize
        do_resize(&mut app, 64, 48);
        assert_eq!(app.canvas.width, 64);

        // Undo should restore original size and content
        app.undo();
        assert_eq!(app.canvas.width, orig_w);
        assert_eq!(app.canvas.height, orig_h);
        assert_eq!(app.canvas.get(10, 10), Some(cell));
    }

    #[test]
    fn test_resize_viewport_reset() {
        let mut app = App::new();
        app.viewport_x = 10;
        app.viewport_y = 5;

        do_resize(&mut app, 64, 48);
        assert_eq!(app.viewport_x, 0);
        assert_eq!(app.viewport_y, 0);
    }

    // --- Import browse tests ---

    #[test]
    fn test_import_browse_opens() {
        let mut app = App::new();
        assert_eq!(app.mode, AppMode::Normal);

        // Simulate Ctrl+I
        handle_event(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::CONTROL)),
            &area(),
        );

        // Should be in ImportBrowse or show status (depends on files in cwd)
        assert!(
            app.mode == AppMode::ImportBrowse
                || app.status_message.is_some(),
            "Ctrl+I should open import dialog or show status"
        );
    }

    #[test]
    fn test_import_browse_filter() {
        // list_import_entries should only return image files and directories
        let dir = std::env::temp_dir().join("kakukuma_test_browse_filter");
        std::fs::create_dir_all(&dir).unwrap();

        // Create test files
        std::fs::write(dir.join("photo.png"), b"fake").unwrap();
        std::fs::write(dir.join("pic.jpg"), b"fake").unwrap();
        std::fs::write(dir.join("data.txt"), b"fake").unwrap();
        std::fs::write(dir.join("notes.md"), b"fake").unwrap();
        std::fs::write(dir.join("image.gif"), b"fake").unwrap();

        let entries = list_import_entries(&dir);

        // Should contain image files but not .txt or .md
        assert!(entries.iter().any(|e| e == "photo.png"));
        assert!(entries.iter().any(|e| e == "pic.jpg"));
        assert!(entries.iter().any(|e| e == "image.gif"));
        assert!(!entries.iter().any(|e| e == "data.txt"));
        assert!(!entries.iter().any(|e| e == "notes.md"));

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_import_browse_stores_path() {
        let dir = std::env::temp_dir().join("kakukuma_test_browse_select");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("test.png"), b"fake").unwrap();

        let mut app = App::new();
        app.import_dir = dir.clone();
        app.file_dialog_files = list_import_entries(&dir);
        app.mode = AppMode::ImportBrowse;

        // Find the index of test.png
        let png_idx = app.file_dialog_files.iter().position(|e| e == "test.png");
        assert!(png_idx.is_some(), "test.png should be in file list");
        app.file_dialog_selected = png_idx.unwrap();

        // Press Enter to select
        handle_import_browse(&mut app, KeyCode::Enter);

        assert_eq!(app.mode, AppMode::ImportOptions);
        assert!(app.import_path.is_some());
        let path = app.import_path.unwrap();
        assert!(path.to_string_lossy().contains("test.png"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    // --- Import options tests ---

    #[test]
    fn test_import_options_navigation() {
        let mut app = App::new();
        app.mode = AppMode::ImportOptions;
        app.import_options_cursor = 0;

        handle_import_options(&mut app, KeyCode::Down);
        assert_eq!(app.import_options_cursor, 1);

        handle_import_options(&mut app, KeyCode::Down);
        assert_eq!(app.import_options_cursor, 2);

        // Can't go past 2
        handle_import_options(&mut app, KeyCode::Down);
        assert_eq!(app.import_options_cursor, 2);

        handle_import_options(&mut app, KeyCode::Up);
        assert_eq!(app.import_options_cursor, 1);

        // Toggle color mode
        assert_eq!(app.import_color, 0);
        handle_import_options(&mut app, KeyCode::Right);
        assert_eq!(app.import_color, 1);
        handle_import_options(&mut app, KeyCode::Left);
        assert_eq!(app.import_color, 0);
    }

    #[test]
    fn test_import_applies_to_canvas() {
        let dir = std::env::temp_dir().join("kakukuma_test_import_apply");
        std::fs::create_dir_all(&dir).unwrap();
        let img_path = dir.join("red_8x8.png");

        // Create an 8x8 red image matching minimum canvas size
        let mut img = image::RgbaImage::new(8, 8);
        for x in 0..8u32 {
            for y in 0..8u32 {
                img.put_pixel(x, y, image::Rgba([255, 0, 0, 255]));
            }
        }
        img.save(&img_path).unwrap();

        let mut app = App::new();
        // Use 8x8 canvas so image fills exactly (no letterbox)
        app.canvas = Canvas::new_with_size(8, 8);
        app.import_path = Some(img_path.clone());
        app.import_charset = 0; // FullBlocks
        app.import_color = 0;   // 256 color
        app.import_fit = 0;     // FitToCanvas

        do_import(&mut app);

        assert_eq!(app.mode, AppMode::Normal);
        // Cell (0,0) should have bg color (from the red image)
        let cell = app.canvas.get(0, 0).unwrap();
        assert!(cell.bg.is_some(), "Canvas cell should have imported color");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_import_undo() {
        let dir = std::env::temp_dir().join("kakukuma_test_import_undo");
        std::fs::create_dir_all(&dir).unwrap();
        let img_path = dir.join("blue_8x8.png");

        let mut img = image::RgbaImage::new(8, 8);
        for x in 0..8u32 {
            for y in 0..8u32 {
                img.put_pixel(x, y, image::Rgba([0, 0, 255, 255]));
            }
        }
        img.save(&img_path).unwrap();

        let mut app = App::new();
        // Use 8x8 canvas so image fills exactly
        app.canvas = Canvas::new_with_size(8, 8);
        let orig_cell = app.canvas.get(0, 0).unwrap();

        app.import_path = Some(img_path.clone());
        app.import_charset = 0;
        app.import_color = 0;
        app.import_fit = 0;
        do_import(&mut app);

        // Canvas should be modified
        let imported_cell = app.canvas.get(0, 0).unwrap();
        assert!(imported_cell.bg.is_some());

        // Undo should restore
        app.undo();
        let restored_cell = app.canvas.get(0, 0).unwrap();
        assert_eq!(restored_cell, orig_cell, "Undo should restore original canvas");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
