use serde::{Deserialize, Serialize};

use crate::cell::Cell;

pub const CANVAS_WIDTH: usize = 32;
pub const CANVAS_HEIGHT: usize = 32;

#[derive(Clone, Serialize, Deserialize)]
pub struct Canvas {
    cells: [[Cell; CANVAS_WIDTH]; CANVAS_HEIGHT],
}

impl Canvas {
    pub fn new() -> Self {
        Canvas {
            cells: [[Cell::default(); CANVAS_WIDTH]; CANVAS_HEIGHT],
        }
    }

    pub fn get(&self, x: usize, y: usize) -> Option<Cell> {
        if x < CANVAS_WIDTH && y < CANVAS_HEIGHT {
            Some(self.cells[y][x])
        } else {
            None
        }
    }

    pub fn set(&mut self, x: usize, y: usize, cell: Cell) {
        if x < CANVAS_WIDTH && y < CANVAS_HEIGHT {
            self.cells[y][x] = cell;
        }
    }

    pub fn clear(&mut self) {
        self.cells = [[Cell::default(); CANVAS_WIDTH]; CANVAS_HEIGHT];
    }
}

impl Default for Canvas {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::{BlockChar, Color256};

    #[test]
    fn test_new_canvas_is_empty() {
        let canvas = Canvas::new();
        for y in 0..CANVAS_HEIGHT {
            for x in 0..CANVAS_WIDTH {
                assert_eq!(canvas.get(x, y), Some(Cell::default()));
            }
        }
    }

    #[test]
    fn test_get_set() {
        let mut canvas = Canvas::new();
        let cell = Cell {
            block: BlockChar::Full,
            fg: Color256(1),
            bg: Color256(4),
        };
        canvas.set(5, 10, cell);
        assert_eq!(canvas.get(5, 10), Some(cell));
    }

    #[test]
    fn test_out_of_bounds_get() {
        let canvas = Canvas::new();
        assert_eq!(canvas.get(32, 0), None);
        assert_eq!(canvas.get(0, 32), None);
        assert_eq!(canvas.get(100, 100), None);
    }

    #[test]
    fn test_out_of_bounds_set() {
        let mut canvas = Canvas::new();
        let cell = Cell {
            block: BlockChar::Full,
            fg: Color256(1),
            bg: Color256::BLACK,
        };
        canvas.set(32, 0, cell); // Should not panic
        canvas.set(0, 32, cell); // Should not panic
    }

    #[test]
    fn test_clear() {
        let mut canvas = Canvas::new();
        let cell = Cell {
            block: BlockChar::Full,
            fg: Color256(1),
            bg: Color256(4),
        };
        canvas.set(0, 0, cell);
        canvas.set(31, 31, cell);
        canvas.clear();
        assert_eq!(canvas.get(0, 0), Some(Cell::default()));
        assert_eq!(canvas.get(31, 31), Some(Cell::default()));
    }
}
