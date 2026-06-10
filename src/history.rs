use crate::canvas::Canvas;
use crate::cell::Cell;

const MAX_HISTORY: usize = 256;

#[derive(Clone, Debug)]
pub struct CellMutation {
    pub x: usize,
    pub y: usize,
    pub old: Cell,
    pub new: Cell,
}

#[derive(Clone)]
pub enum Action {
    /// Per-cell mutations (pencil, eraser, fill, line, rect).
    CellChange(Vec<CellMutation>),
    /// Whole-canvas snapshot (resize, import).
    CanvasSnapshot {
        old_cells: Vec<Vec<Cell>>,
        old_w: usize,
        old_h: usize,
        new_cells: Vec<Vec<Cell>>,
        new_w: usize,
        new_h: usize,
    },
}

pub struct History {
    undo_stack: Vec<Action>,
    redo_stack: Vec<Action>,
    pending: Option<Vec<CellMutation>>,
}

impl History {
    pub fn new() -> Self {
        History {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            pending: None,
        }
    }

    /// Start accumulating mutations for a drag stroke.
    pub fn begin_stroke(&mut self) {
        if self.pending.is_some() {
            self.end_stroke();
        }
        self.pending = Some(Vec::new());
    }

    /// Add a mutation to the current pending stroke.
    /// If no stroke is active, commits immediately as a single action.
    pub fn push_mutation(&mut self, mutation: CellMutation) {
        if let Some(ref mut pending) = self.pending {
            pending.push(mutation);
        } else {
            self.commit(Action::CellChange(vec![mutation]));
        }
    }

    /// Finish the current drag stroke and commit it as one action.
    pub fn end_stroke(&mut self) {
        if let Some(mutations) = self.pending.take() {
            if !mutations.is_empty() {
                self.commit(Action::CellChange(mutations));
            }
        }
    }

    /// Commit an action to the undo stack.
    pub fn commit(&mut self, action: Action) {
        match &action {
            Action::CellChange(mutations) if mutations.is_empty() => return,
            _ => {}
        }
        self.redo_stack.clear();
        self.undo_stack.push(action);
        if self.undo_stack.len() > MAX_HISTORY {
            self.undo_stack.remove(0);
        }
    }

    /// Undo the last action, applying old cell values.
    pub fn undo(&mut self, canvas: &mut Canvas) -> bool {
        if let Some(action) = self.undo_stack.pop() {
            match &action {
                Action::CellChange(mutations) => {
                    for m in mutations.iter().rev() {
                        canvas.set(m.x, m.y, m.old);
                    }
                }
                Action::CanvasSnapshot { old_cells, old_w, old_h, .. } => {
                    canvas.replace(old_cells.clone(), *old_w, *old_h);
                }
            }
            self.redo_stack.push(action);
            true
        } else {
            false
        }
    }

    /// Redo the last undone action, applying new cell values.
    pub fn redo(&mut self, canvas: &mut Canvas) -> bool {
        if let Some(action) = self.redo_stack.pop() {
            match &action {
                Action::CellChange(mutations) => {
                    for m in mutations {
                        canvas.set(m.x, m.y, m.new);
                    }
                }
                Action::CanvasSnapshot { new_cells, new_w, new_h, .. } => {
                    canvas.replace(new_cells.clone(), *new_w, *new_h);
                }
            }
            self.undo_stack.push(action);
            true
        } else {
            false
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn is_stroke_active(&self) -> bool {
        self.pending.is_some()
    }
}

impl Default for History {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::{blocks, Cell, Rgb};

    fn red_cell() -> Cell {
        Cell {
            ch: blocks::FULL,
            fg: Some(Rgb { r: 205, g: 0, b: 0 }),
            bg: None,
        }
    }

    #[test]
    fn test_undo_redo_single() {
        let mut canvas = Canvas::new();
        let mut history = History::new();

        let old = canvas.get(0, 0).unwrap();
        let new = red_cell();
        canvas.set(0, 0, new);
        history.push_mutation(CellMutation {
            x: 0,
            y: 0,
            old,
            new,
        });

        assert_eq!(canvas.get(0, 0), Some(new));
        history.undo(&mut canvas);
        assert_eq!(canvas.get(0, 0), Some(old));
        history.redo(&mut canvas);
        assert_eq!(canvas.get(0, 0), Some(new));
    }

    #[test]
    fn test_stroke_batching() {
        let mut canvas = Canvas::new();
        let mut history = History::new();

        history.begin_stroke();
        for x in 0..5 {
            let old = canvas.get(x, 0).unwrap();
            let new = red_cell();
            canvas.set(x, 0, new);
            history.push_mutation(CellMutation {
                x,
                y: 0,
                old,
                new,
            });
        }
        history.end_stroke();

        // One undo should revert all 5 cells
        history.undo(&mut canvas);
        for x in 0..5 {
            assert_eq!(canvas.get(x, 0), Some(Cell::default()));
        }
    }

    #[test]
    fn test_new_action_clears_redo() {
        let mut canvas = Canvas::new();
        let mut history = History::new();

        let old = canvas.get(0, 0).unwrap();
        let new = red_cell();
        canvas.set(0, 0, new);
        history.push_mutation(CellMutation {
            x: 0,
            y: 0,
            old,
            new,
        });
        history.undo(&mut canvas);
        assert!(history.can_redo());

        // New action should clear redo
        let old2 = canvas.get(1, 1).unwrap();
        canvas.set(1, 1, new);
        history.push_mutation(CellMutation {
            x: 1,
            y: 1,
            old: old2,
            new,
        });
        assert!(!history.can_redo());
    }

    #[test]
    fn test_capacity_limit() {
        let mut canvas = Canvas::new();
        let mut history = History::new();

        for i in 0..300 {
            let x = i % 32;
            let old = canvas.get(x, 0).unwrap();
            let new = red_cell();
            canvas.set(x, 0, new);
            history.push_mutation(CellMutation {
                x,
                y: 0,
                old,
                new,
            });
        }

        // Should have at most MAX_HISTORY (256) actions
        let mut count = 0;
        while history.undo(&mut canvas) {
            count += 1;
        }
        assert!(count <= 256);
    }

    // --- Cycle 15 QA: Shade character undo test ---

    #[test]
    fn test_undo_shade_placement() {
        use crate::cell::blocks;

        let mut canvas = Canvas::new();
        let mut history = History::new();

        let old = canvas.get(4, 6).unwrap();
        let new = Cell {
            ch: blocks::SHADE_DARK,
            fg: Some(Rgb { r: 0, g: 205, b: 0 }),
            bg: None,
        };
        canvas.set(4, 6, new);
        history.push_mutation(CellMutation {
            x: 4,
            y: 6,
            old,
            new,
        });

        // Verify shade was placed
        assert_eq!(canvas.get(4, 6).unwrap().ch, blocks::SHADE_DARK);

        // Undo should revert to original empty cell
        assert!(history.undo(&mut canvas));
        let reverted = canvas.get(4, 6).unwrap();
        assert_eq!(reverted.ch, ' ');
        assert_eq!(reverted, Cell::default());

        // Redo should restore the shade
        assert!(history.redo(&mut canvas));
        assert_eq!(canvas.get(4, 6).unwrap().ch, blocks::SHADE_DARK);
    }

    // --- Cycle 16: CanvasSnapshot tests ---

    #[test]
    fn test_canvas_snapshot_undo() {
        let mut canvas = Canvas::new_with_size(16, 16);
        let mut history = History::new();

        // Place a cell before resize
        let cell = red_cell();
        canvas.set(5, 5, cell);

        // Capture old state
        let old_cells = canvas.cells();
        let old_w = canvas.width;
        let old_h = canvas.height;

        // Resize
        canvas.resize(32, 32);

        // Capture new state
        let new_cells = canvas.cells();
        let new_w = canvas.width;
        let new_h = canvas.height;

        history.commit(Action::CanvasSnapshot {
            old_cells, old_w, old_h,
            new_cells, new_w, new_h,
        });

        assert_eq!(canvas.width, 32);
        assert_eq!(canvas.height, 32);

        // Undo restores original 16x16
        assert!(history.undo(&mut canvas));
        assert_eq!(canvas.width, 16);
        assert_eq!(canvas.height, 16);
        assert_eq!(canvas.get(5, 5), Some(cell));
    }

    #[test]
    fn test_canvas_snapshot_redo() {
        let mut canvas = Canvas::new_with_size(16, 16);
        let mut history = History::new();

        canvas.set(5, 5, red_cell());

        let old_cells = canvas.cells();
        let old_w = canvas.width;
        let old_h = canvas.height;

        canvas.resize(32, 32);

        let new_cells = canvas.cells();
        let new_w = canvas.width;
        let new_h = canvas.height;

        history.commit(Action::CanvasSnapshot {
            old_cells, old_w, old_h,
            new_cells: new_cells.clone(), new_w, new_h,
        });

        // Undo then redo
        history.undo(&mut canvas);
        assert_eq!(canvas.width, 16);

        assert!(history.redo(&mut canvas));
        assert_eq!(canvas.width, 32);
        assert_eq!(canvas.height, 32);
        // Original cell preserved at (5,5)
        assert_eq!(canvas.get(5, 5), Some(red_cell()));
    }

    #[test]
    fn test_mixed_history() {
        let mut canvas = Canvas::new_with_size(16, 16);
        let mut history = History::new();

        // 1. Paint a cell
        let old = canvas.get(3, 3).unwrap();
        let cell = red_cell();
        canvas.set(3, 3, cell);
        history.push_mutation(CellMutation { x: 3, y: 3, old, new: cell });

        // 2. Resize via snapshot
        let old_cells = canvas.cells();
        canvas.resize(24, 24);
        let new_cells = canvas.cells();
        history.commit(Action::CanvasSnapshot {
            old_cells, old_w: 16, old_h: 16,
            new_cells, new_w: 24, new_h: 24,
        });

        assert_eq!(canvas.width, 24);

        // Undo resize → back to 16x16 with red cell
        history.undo(&mut canvas);
        assert_eq!(canvas.width, 16);
        assert_eq!(canvas.get(3, 3), Some(cell));

        // Undo paint → back to default cell
        history.undo(&mut canvas);
        assert_eq!(canvas.get(3, 3), Some(Cell::default()));
    }
}
