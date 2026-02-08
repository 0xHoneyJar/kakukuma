use std::path::{Path, PathBuf};

use crate::canvas::Canvas;
use crate::cell::Color256;
use crate::cell::BlockChar;
use crate::export;
use crate::history::{CellMutation, History};
use crate::project::Project;
use crate::symmetry::{self, SymmetryMode};
use crate::palette::{self, HueGroup};
use crate::tools::{self, ToolKind, ToolState};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AppMode {
    Normal,
    ExportDialog,
    FileDialog,
    SaveAs,
    ExportFile,
    Help,
    Quitting,
    Recovery,
    ColorSliders,
    PaletteDialog,
    PaletteNameInput,
}

pub struct StatusMessage {
    pub text: String,
    pub ticks_remaining: u16,
}

pub struct App {
    pub canvas: Canvas,
    pub active_tool: ToolKind,
    pub color: Color256,
    pub symmetry: SymmetryMode,
    pub history: History,
    pub cursor: Option<(usize, usize)>,
    pub show_grid: bool,
    pub show_preview: bool,
    pub tool_state: ToolState,
    pub mode: AppMode,
    pub dirty: bool,
    pub status_message: Option<StatusMessage>,
    pub running: bool,
    pub project_name: Option<String>,
    pub project_path: Option<String>,
    pub filled_rect: bool,
    // File dialog state
    pub file_dialog_files: Vec<String>,
    pub file_dialog_selected: usize,
    // Export dialog state: 0=PlainText, 1=ANSI
    pub export_format: usize,
    // Export dialog state: 0=Clipboard, 1=File
    pub export_dest: usize,
    // Export dialog cursor row: 0=format, 1=dest
    pub export_cursor: usize,
    // Shared text input for SaveAs and ExportFile modes
    pub text_input: String,
    // Auto-save tick counter (increments each tick, resets on save)
    pub auto_save_ticks: u16,
    // Path of autosave file found on startup
    pub recovery_path: Option<String>,
    // Recent colors (auto-tracked, last 8 unique)
    pub recent_colors: Vec<Color256>,
    // Palette browser state
    pub hue_groups: Vec<HueGroup>,
    pub palette_scroll: usize,
    pub palette_cursor: usize,
    // HSL slider state
    pub slider_h: u16,
    pub slider_s: u8,
    pub slider_l: u8,
    pub slider_active: u8, // 0=H, 1=S, 2=L
    // Custom palette state
    pub custom_palette: Option<palette::CustomPalette>,
    pub palette_dialog_files: Vec<String>,
    pub palette_dialog_selected: usize,
}

impl App {
    pub fn new() -> Self {
        App {
            canvas: Canvas::new(),
            active_tool: ToolKind::Pencil,
            color: Color256::WHITE,
            symmetry: SymmetryMode::Off,
            history: History::new(),
            cursor: None,
            show_grid: true,
            show_preview: false,
            tool_state: ToolState::Idle,
            mode: AppMode::Normal,
            dirty: false,
            status_message: None,
            running: true,
            project_name: None,
            project_path: None,
            filled_rect: false,
            file_dialog_files: Vec::new(),
            file_dialog_selected: 0,
            export_format: 0,
            export_dest: 0,
            export_cursor: 0,
            text_input: String::new(),
            auto_save_ticks: 0,
            recovery_path: None,
            recent_colors: Vec::new(),
            hue_groups: palette::build_hue_groups(),
            palette_scroll: 0,
            palette_cursor: 0,
            slider_h: 0,
            slider_s: 0,
            slider_l: 50,
            slider_active: 0,
            custom_palette: None,
            palette_dialog_files: Vec::new(),
            palette_dialog_selected: 0,
        }
    }

    pub fn set_status(&mut self, msg: &str) {
        self.status_message = Some(StatusMessage {
            text: msg.to_string(),
            ticks_remaining: 30, // ~3 seconds at 10 ticks/sec
        });
    }

    pub fn tick_status(&mut self) {
        if let Some(ref mut msg) = self.status_message {
            if msg.ticks_remaining > 0 {
                msg.ticks_remaining -= 1;
            } else {
                self.status_message = None;
            }
        }
    }

    /// Get the flattened list of all browsable palette colors.
    pub fn all_palette_colors(&self) -> Vec<u8> {
        let mut colors = Vec::new();

        // Custom palette colors at top when loaded
        if let Some(ref cp) = self.custom_palette {
            colors.extend(&cp.colors);
        }

        colors.extend(palette::all_palette_colors(&self.recent_colors, &self.hue_groups));
        colors
    }

    /// Ensure palette_scroll keeps the cursor visible in a given viewport height.
    pub fn ensure_palette_cursor_visible(&mut self, viewport_height: usize) {
        // Approximate: each color row holds COLS=6 items, plus section headers.
        // Rough estimate: cursor_line ≈ palette_cursor / 6 + (section headers before it)
        // For simplicity, use palette_cursor / 6 as the line estimate with padding.
        let estimated_line = self.palette_cursor / 6;
        if estimated_line < self.palette_scroll {
            self.palette_scroll = estimated_line;
        } else if estimated_line >= self.palette_scroll + viewport_height.saturating_sub(2) {
            self.palette_scroll = estimated_line.saturating_sub(viewport_height.saturating_sub(4));
        }
    }

    /// Track a color in the recent colors list.
    fn track_recent_color(&mut self, color: Color256) {
        // Remove if already present (to move it to front)
        self.recent_colors.retain(|&c| c != color);
        // Push to front
        self.recent_colors.insert(0, color);
        // Cap at 8
        self.recent_colors.truncate(8);
    }

    /// Apply a tool action at (x, y), handling symmetry and history.
    pub fn apply_tool(&mut self, x: usize, y: usize) {
        let fg = self.color;
        let bg = Color256::BLACK;
        let mutations = match self.active_tool {
            ToolKind::Pencil => {
                self.track_recent_color(fg);
                tools::pencil(&self.canvas, x, y, BlockChar::Full, fg, bg)
            }
            ToolKind::Eraser => tools::eraser(&self.canvas, x, y),
            ToolKind::Fill => {
                self.track_recent_color(fg);
                tools::flood_fill(&self.canvas, x, y, BlockChar::Full, fg, bg)
            }
            ToolKind::Eyedropper => {
                if let Some((picked, _bg, _block)) = tools::eyedropper(&self.canvas, x, y) {
                    self.color = picked;
                    self.track_recent_color(picked);
                    self.set_status(&format!("Picked: {}", picked.name()));
                }
                return;
            }
            ToolKind::Line => {
                match self.tool_state.clone() {
                    ToolState::Idle => {
                        self.tool_state = ToolState::LineStart { x, y };
                        self.set_status("Line: click endpoint");
                        return;
                    }
                    ToolState::LineStart { x: x0, y: y0 } => {
                        self.tool_state = ToolState::Idle;
                        self.track_recent_color(fg);
                        tools::line(&self.canvas, x0, y0, x, y, BlockChar::Full, fg, bg)
                    }
                    _ => return,
                }
            }
            ToolKind::Rectangle => {
                match self.tool_state.clone() {
                    ToolState::Idle => {
                        self.tool_state = ToolState::RectStart { x, y };
                        self.set_status("Rect: click second corner");
                        return;
                    }
                    ToolState::RectStart { x: x0, y: y0 } => {
                        self.tool_state = ToolState::Idle;
                        self.track_recent_color(fg);
                        tools::rectangle(
                            &self.canvas, x0, y0, x, y, BlockChar::Full, fg, bg,
                            self.filled_rect,
                        )
                    }
                    _ => return,
                }
            }
        };

        // Apply symmetry
        let mutations = symmetry::apply_symmetry(mutations, self.symmetry);

        if mutations.is_empty() {
            return;
        }

        // Read old values for mirrored cells (symmetry mutations have wrong `old` values
        // since they were cloned from the original mutation)
        let mutations: Vec<CellMutation> = mutations
            .into_iter()
            .map(|mut m| {
                if let Some(actual_old) = self.canvas.get(m.x, m.y) {
                    m.old = actual_old;
                }
                m
            })
            .collect();

        // Apply to canvas
        for m in &mutations {
            self.canvas.set(m.x, m.y, m.new);
        }

        // Record in history
        for m in mutations {
            self.history.push_mutation(m);
        }

        self.dirty = true;
    }

    pub fn begin_stroke(&mut self) {
        self.history.begin_stroke();
    }

    pub fn end_stroke(&mut self) {
        self.history.end_stroke();
    }

    pub fn undo(&mut self) {
        if self.history.undo(&mut self.canvas) {
            self.dirty = true;
            self.set_status("Undo");
        }
    }

    pub fn redo(&mut self) {
        if self.history.redo(&mut self.canvas) {
            self.dirty = true;
            self.set_status("Redo");
        }
    }

    pub fn cancel_tool(&mut self) {
        self.tool_state = ToolState::Idle;
    }

    /// Open the custom palette dialog, scanning for .palette files.
    pub fn open_palette_dialog(&mut self) {
        let cwd = std::env::current_dir().unwrap_or_default();
        self.palette_dialog_files = palette::list_palette_files(&cwd);
        self.palette_dialog_selected = 0;
        self.mode = AppMode::PaletteDialog;
    }

    /// Load the currently selected palette from the dialog.
    pub fn load_selected_palette(&mut self) {
        if let Some(filename) = self.palette_dialog_files.get(self.palette_dialog_selected).cloned() {
            match palette::load_palette(Path::new(&filename)) {
                Ok(cp) => {
                    self.set_status(&format!("Loaded palette: {}", cp.name));
                    self.custom_palette = Some(cp);
                    self.mode = AppMode::Normal;
                }
                Err(e) => {
                    self.set_status(&format!("Load failed: {}", e));
                }
            }
        }
    }

    /// Delete the currently selected palette file.
    pub fn delete_selected_palette(&mut self) {
        if let Some(filename) = self.palette_dialog_files.get(self.palette_dialog_selected).cloned() {
            match std::fs::remove_file(&filename) {
                Ok(()) => {
                    self.set_status(&format!("Deleted: {}", filename));
                    // If this was the loaded palette, unload it
                    if let Some(ref cp) = self.custom_palette {
                        let expected = format!("{}.palette", cp.name);
                        if filename == expected {
                            self.custom_palette = None;
                        }
                    }
                    // Refresh file list
                    let cwd = std::env::current_dir().unwrap_or_default();
                    self.palette_dialog_files = palette::list_palette_files(&cwd);
                    if self.palette_dialog_selected >= self.palette_dialog_files.len() && self.palette_dialog_selected > 0 {
                        self.palette_dialog_selected -= 1;
                    }
                }
                Err(e) => {
                    self.set_status(&format!("Delete failed: {}", e));
                }
            }
        }
    }

    /// Create a new custom palette with the given name.
    pub fn create_custom_palette(&mut self, name: &str) {
        let cp = palette::CustomPalette {
            name: name.to_string(),
            colors: Vec::new(),
        };
        let filename = format!("{}.palette", name);
        match palette::save_palette(&cp, Path::new(&filename)) {
            Ok(()) => {
                self.set_status(&format!("Created palette: {}", name));
                self.custom_palette = Some(cp);
                self.mode = AppMode::Normal;
            }
            Err(e) => {
                self.set_status(&format!("Create failed: {}", e));
                self.mode = AppMode::Normal;
            }
        }
    }

    /// Add the current color to the active custom palette and auto-save.
    pub fn add_color_to_custom_palette(&mut self) {
        let color = self.color;
        match self.custom_palette {
            Some(ref mut cp) => {
                let idx = color.0;
                if !cp.colors.contains(&idx) {
                    cp.colors.push(idx);
                    let filename = format!("{}.palette", cp.name);
                    let _ = palette::save_palette(cp, Path::new(&filename));
                    let msg = format!("Added {} to {}", color.name(), cp.name);
                    self.set_status(&msg);
                } else {
                    self.set_status("Color already in palette");
                }
            }
            None => {
                self.set_status("No palette loaded. Press C to open palettes.");
            }
        }
    }

    /// Save the current project to its path. If no path, returns false (need SaveAs).
    pub fn save_project(&mut self) -> bool {
        let path = match &self.project_path {
            Some(p) => PathBuf::from(p),
            None => return false,
        };
        let name = self.project_name.clone().unwrap_or_else(|| "untitled".to_string());
        let mut project = Project::new(
            &name,
            self.canvas.clone(),
            self.color,
            self.symmetry,
        );
        match project.save_to_file(&path) {
            Ok(()) => {
                self.dirty = false;
                self.auto_save_ticks = 0;
                // Delete autosave file if it exists
                let autosave = format!("{}.autosave", path.display());
                let _ = std::fs::remove_file(&autosave);
                self.set_status("Saved!");
                true
            }
            Err(e) => {
                self.set_status(&format!("Save failed: {}", e));
                false
            }
        }
    }

    /// Save with a specific name (from SaveAs dialog).
    pub fn save_as(&mut self, name: &str) {
        let filename = if name.ends_with(".kaku") {
            name.to_string()
        } else {
            format!("{}.kaku", name)
        };
        self.project_name = Some(name.trim_end_matches(".kaku").to_string());
        self.project_path = Some(filename);
        self.save_project();
    }

    /// Load a project from a .kaku file.
    pub fn load_project(&mut self, filename: &str) {
        let path = Path::new(filename);
        match Project::load_from_file(path) {
            Ok(project) => {
                self.canvas = project.canvas;
                self.color = project.color;
                self.symmetry = project.symmetry;
                self.project_name = Some(project.name);
                self.project_path = Some(filename.to_string());
                self.dirty = false;
                self.history = History::new();
                self.auto_save_ticks = 0;
                self.set_status(&format!("Opened: {}", filename));
            }
            Err(e) => {
                self.set_status(&format!("Load failed: {}", e));
            }
        }
    }

    /// Create a new blank project (Ctrl+N).
    pub fn new_project(&mut self) {
        self.canvas.clear();
        self.color = Color256::WHITE;
        self.symmetry = SymmetryMode::Off;
        self.history = History::new();
        self.dirty = false;
        self.project_name = None;
        self.project_path = None;
        self.auto_save_ticks = 0;
        self.set_status("New project");
    }

    /// Populate file dialog with .kaku files from current directory.
    pub fn open_file_dialog(&mut self) {
        let cwd = std::env::current_dir().unwrap_or_default();
        self.file_dialog_files = crate::project::list_kaku_files(&cwd);
        self.file_dialog_selected = 0;
        if self.file_dialog_files.is_empty() {
            self.set_status("No .kaku files found");
        } else {
            self.mode = AppMode::FileDialog;
        }
    }

    /// Execute the current export dialog selection.
    pub fn do_export(&mut self) {
        let content = if self.export_format == 0 {
            export::to_plain_text(&self.canvas)
        } else {
            export::to_ansi(&self.canvas)
        };

        if self.export_dest == 0 {
            // Clipboard
            match arboard::Clipboard::new() {
                Ok(mut clipboard) => match clipboard.set_text(&content) {
                    Ok(()) => {
                        self.set_status("Copied to clipboard!");
                        self.mode = AppMode::Normal;
                    }
                    Err(e) => {
                        self.set_status(&format!("Clipboard error: {}", e));
                        self.mode = AppMode::Normal;
                    }
                },
                Err(e) => {
                    self.set_status(&format!("Clipboard unavailable: {}. Use File export.", e));
                    self.mode = AppMode::Normal;
                }
            }
        } else {
            // File — switch to text input for filename
            let ext = if self.export_format == 0 { "txt" } else { "ans" };
            let base = self
                .project_name
                .as_deref()
                .unwrap_or("untitled");
            self.text_input = format!("{}.{}", base, ext);
            self.mode = AppMode::ExportFile;
        }
    }

    /// Write export content to a file.
    pub fn export_to_file(&mut self, filename: &str) {
        let content = if self.export_format == 0 {
            export::to_plain_text(&self.canvas)
        } else {
            export::to_ansi(&self.canvas)
        };
        match std::fs::write(filename, &content) {
            Ok(()) => self.set_status(&format!("Exported to {}", filename)),
            Err(e) => self.set_status(&format!("Export failed: {}", e)),
        }
        self.mode = AppMode::Normal;
    }

    /// Auto-save tick. Call each event loop iteration (~100ms).
    /// Triggers auto-save after 600 ticks (60 seconds) if dirty.
    pub fn tick_auto_save(&mut self) {
        if !self.dirty {
            return;
        }
        self.auto_save_ticks += 1;
        if self.auto_save_ticks >= 600 {
            self.auto_save_ticks = 0;
            self.do_auto_save();
        }
    }

    fn do_auto_save(&mut self) {
        let path = match &self.project_path {
            Some(p) => format!("{}.autosave", p),
            None => "untitled.kaku.autosave".to_string(),
        };
        let name = self.project_name.clone().unwrap_or_else(|| "untitled".to_string());
        let mut project = Project::new(
            &name,
            self.canvas.clone(),
            self.color,
            self.symmetry,
        );
        if project.save_to_file(Path::new(&path)).is_ok() {
            self.set_status("Auto-saved");
        }
    }

    /// Check for autosave files on startup and prompt recovery.
    pub fn check_recovery(&mut self) {
        let cwd = std::env::current_dir().unwrap_or_default();
        if let Some(autosave_name) = crate::project::find_autosave(&cwd) {
            self.recovery_path = Some(autosave_name);
            self.mode = AppMode::Recovery;
        }
    }

    /// Recover from an autosave file.
    pub fn recover_autosave(&mut self) {
        if let Some(ref autosave) = self.recovery_path.clone() {
            let path = Path::new(autosave);
            match Project::load_from_file(path) {
                Ok(project) => {
                    self.canvas = project.canvas;
                    self.color = project.color;
                    self.symmetry = project.symmetry;
                    self.project_name = Some(project.name);
                    // Derive the real save path from autosave name
                    let real_path = autosave.trim_end_matches(".autosave");
                    if !real_path.is_empty() && real_path != "untitled.kaku" {
                        self.project_path = Some(real_path.to_string());
                    }
                    self.dirty = true; // Mark dirty so user knows to save properly
                    self.set_status("Recovered from autosave");
                }
                Err(e) => {
                    self.set_status(&format!("Recovery failed: {}", e));
                }
            }
        }
        self.recovery_path = None;
        self.mode = AppMode::Normal;
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
