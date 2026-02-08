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
pub struct Action {
    pub mutations: Vec<CellMutation>,
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
        self.pending = Some(Vec::new());
    }

    /// Add a mutation to the current pending stroke.
    /// If no stroke is active, commits immediately as a single action.
    pub fn push_mutation(&mut self, mutation: CellMutation) {
        if let Some(ref mut pending) = self.pending {
            pending.push(mutation);
        } else {
            self.commit(Action {
                mutations: vec![mutation],
            });
        }
    }

    /// Finish the current drag stroke and commit it as one action.
    pub fn end_stroke(&mut self) {
        if let Some(mutations) = self.pending.take() {
            if !mutations.is_empty() {
                self.commit(Action { mutations });
            }
        }
    }

    /// Commit an action to the undo stack.
    pub fn commit(&mut self, action: Action) {
        if action.mutations.is_empty() {
            return;
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
            for m in action.mutations.iter().rev() {
                canvas.set(m.x, m.y, m.old);
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
            for m in &action.mutations {
                canvas.set(m.x, m.y, m.new);
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
    use crate::cell::{BlockChar, Cell, Color256};

    fn red_cell() -> Cell {
        Cell {
            block: BlockChar::Full,
            fg: Color256(1),
            bg: Color256::BLACK,
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
}
