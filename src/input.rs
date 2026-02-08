use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use crate::app::{App, AppMode};
use crate::cell::Color256;
use crate::tools::ToolKind;

/// Canvas area position in terminal coordinates.
/// Set by the UI renderer each frame.
pub struct CanvasArea {
    pub left: u16,
    pub top: u16,
    pub width: u16,
    pub height: u16,
}

impl CanvasArea {
    /// Convert screen coordinates to canvas cell coordinates.
    /// Returns None if outside canvas bounds.
    pub fn screen_to_canvas(&self, screen_x: u16, screen_y: u16) -> Option<(usize, usize)> {
        if screen_x < self.left || screen_y < self.top {
            return None;
        }
        let rel_x = screen_x - self.left;
        let rel_y = screen_y - self.top;
        if rel_x >= self.width || rel_y >= self.height {
            return None;
        }
        // Each canvas cell is 2 chars wide
        let canvas_x = (rel_x / 2) as usize;
        let canvas_y = rel_y as usize;
        if canvas_x < 32 && canvas_y < 32 {
            Some((canvas_x, canvas_y))
        } else {
            None
        }
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
                // New project
                if app.dirty {
                    app.mode = AppMode::Quitting;
                    app.set_status("Unsaved changes. Quit? (y/n)");
                } else {
                    app.new_project();
                }
                return;
            }
            KeyCode::Char('e') => {
                // Export dialog
                app.export_format = 0;
                app.export_dest = 0;
                app.export_cursor = 0;
                app.mode = AppMode::ExportDialog;
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

        // Grid toggle
        KeyCode::Char('g') | KeyCode::Char('G') => {
            app.show_grid = !app.show_grid;
            app.set_status(if app.show_grid { "Grid: On" } else { "Grid: Off" });
        }

        // Preview toggle
        KeyCode::Tab => {
            app.show_preview = !app.show_preview;
        }

        // Color selection by number: 1-9 → indices 0-8, 0 → index 9
        KeyCode::Char(c @ '1'..='9') => {
            let idx = (c as u8) - b'1';
            app.color = Color256(idx);
        }
        KeyCode::Char('0') => {
            app.color = Color256(9);
        }

        _ if key.modifiers.contains(KeyModifiers::SHIFT) => {
            match key.code {
                // Shift+1 through Shift+6 for colors 10-15
                KeyCode::Char('!') => { app.color = Color256(10); }
                KeyCode::Char('@') => { app.color = Color256(11); }
                KeyCode::Char('#') => { app.color = Color256(12); }
                KeyCode::Char('$') => { app.color = Color256(13); }
                KeyCode::Char('%') => { app.color = Color256(14); }
                KeyCode::Char('^') => { app.color = Color256(15); }
                _ => {}
            }
        }

        // Palette navigation
        KeyCode::Up => {
            let all = app.all_palette_colors();
            if !all.is_empty() && app.palette_cursor > 0 {
                app.palette_cursor -= 1;
                app.color = Color256(all[app.palette_cursor]);
                app.ensure_palette_cursor_visible(32);
            }
        }
        KeyCode::Down => {
            let all = app.all_palette_colors();
            if !all.is_empty() && app.palette_cursor + 1 < all.len() {
                app.palette_cursor += 1;
                app.color = Color256(all[app.palette_cursor]);
                app.ensure_palette_cursor_visible(32);
            }
        }
        KeyCode::Left => {
            let all = app.all_palette_colors();
            if !all.is_empty() && app.palette_cursor >= 6 {
                app.palette_cursor -= 6;
                app.color = Color256(all[app.palette_cursor]);
                app.ensure_palette_cursor_visible(32);
            }
        }
        KeyCode::Right => {
            let all = app.all_palette_colors();
            if !all.is_empty() && app.palette_cursor + 6 < all.len() {
                app.palette_cursor += 6;
                app.color = Color256(all[app.palette_cursor]);
                app.ensure_palette_cursor_visible(32);
            }
        }

        // HSL color sliders
        KeyCode::Char('s') | KeyCode::Char('S') => {
            let (r, g, b) = app.color.to_rgb();
            let (h, s, l) = crate::palette::rgb_to_hsl(r, g, b);
            app.slider_h = h;
            app.slider_s = s;
            app.slider_l = l;
            app.slider_active = 0;
            app.mode = AppMode::ColorSliders;
        }

        // Custom palette dialog
        KeyCode::Char('c') | KeyCode::Char('C') => {
            app.open_palette_dialog();
        }

        // Add current color to active custom palette
        KeyCode::Char('a') | KeyCode::Char('A') => {
            app.add_color_to_custom_palette();
        }

        // Toggle filled/outline rectangle
        KeyCode::Char('t') | KeyCode::Char('T') => {
            app.filled_rect = !app.filled_rect;
            app.set_status(if app.filled_rect { "Rect: Filled" } else { "Rect: Outline" });
        }

        // Cancel multi-click tool
        KeyCode::Esc => {
            app.cancel_tool();
            app.set_status("Cancelled");
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
    match code {
        KeyCode::Up => {
            if app.export_cursor > 0 {
                app.export_cursor -= 1;
            }
        }
        KeyCode::Down => {
            if app.export_cursor < 1 {
                app.export_cursor += 1;
            }
        }
        KeyCode::Left | KeyCode::Right => {
            if app.export_cursor == 0 {
                app.export_format = 1 - app.export_format;
            } else {
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
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
        }
        _ => {}
    }
}

fn handle_mouse(app: &mut App, mouse: MouseEvent, canvas_area: &CanvasArea) {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if let Some((x, y)) = canvas_area.screen_to_canvas(mouse.column, mouse.row) {
                app.cursor = Some((x, y));
                // Start stroke for continuous tools
                if matches!(app.active_tool, ToolKind::Pencil | ToolKind::Eraser) {
                    app.begin_stroke();
                }
                app.apply_tool(x, y);
            }
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if let Some((x, y)) = canvas_area.screen_to_canvas(mouse.column, mouse.row) {
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
            if let Some((x, y)) = canvas_area.screen_to_canvas(mouse.column, mouse.row) {
                if let Some((picked, _bg, _block)) = crate::tools::eyedropper(&app.canvas, x, y) {
                    app.color = picked;
                    app.set_status(&format!("Picked: {}", picked.name()));
                }
            }
        }
        MouseEventKind::Moved => {
            if let Some((x, y)) = canvas_area.screen_to_canvas(mouse.column, mouse.row) {
                app.cursor = Some((x, y));
            } else {
                app.cursor = None;
            }
        }
        _ => {}
    }
}
