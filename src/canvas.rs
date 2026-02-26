use serde::{Deserialize, Serialize};

use crate::cell::Cell;

pub const DEFAULT_WIDTH: usize = 48;
pub const DEFAULT_HEIGHT: usize = 32;
pub const MIN_DIMENSION: usize = 8;
pub const MAX_DIMENSION: usize = 128;

fn default_width() -> usize { DEFAULT_WIDTH }
fn default_height() -> usize { DEFAULT_HEIGHT }

#[derive(Clone, Serialize, Deserialize)]
pub struct Canvas {
    cells: Vec<Vec<Cell>>,
    #[serde(default = "default_width")]
    pub width: usize,
    #[serde(default = "default_height")]
    pub height: usize,
}

impl Canvas {
    pub fn new() -> Self {
        Self::new_with_size(DEFAULT_WIDTH, DEFAULT_HEIGHT)
    }

    pub fn new_with_size(width: usize, height: usize) -> Self {
        let w = width.clamp(MIN_DIMENSION, MAX_DIMENSION);
        let h = height.clamp(MIN_DIMENSION, MAX_DIMENSION);
        Canvas {
            cells: vec![vec![Cell::default(); w]; h],
            width: w,
            height: h,
        }
    }

    pub fn get(&self, x: usize, y: usize) -> Option<Cell> {
        if x < self.width && y < self.height {
            Some(self.cells[y][x])
        } else {
            None
        }
    }

    pub fn set(&mut self, x: usize, y: usize, cell: Cell) {
        if x < self.width && y < self.height {
            self.cells[y][x] = cell;
        }
    }

    pub fn clear(&mut self) {
        self.cells = vec![vec![Cell::default(); self.width]; self.height];
    }

    /// Clone the entire cell grid (for history snapshots).
    pub fn cells(&self) -> Vec<Vec<Cell>> {
        self.cells.clone()
    }

    /// Replace the entire cell grid and dimensions (for history snapshot restore).
    pub fn replace(&mut self, cells: Vec<Vec<Cell>>, width: usize, height: usize) {
        self.cells = cells;
        self.width = width;
        self.height = height;
    }

    /// Resize the canvas, preserving existing content where it overlaps.
    pub fn resize(&mut self, new_width: usize, new_height: usize) {
        let w = new_width.clamp(MIN_DIMENSION, MAX_DIMENSION);
        let h = new_height.clamp(MIN_DIMENSION, MAX_DIMENSION);
        let mut new_cells = vec![vec![Cell::default(); w]; h];
        let copy_w = w.min(self.width);
        let copy_h = h.min(self.height);
        for (y, new_row) in new_cells.iter_mut().enumerate().take(copy_h) {
            new_row[..copy_w].copy_from_slice(&self.cells[y][..copy_w]);
        }
        self.cells = new_cells;
        self.width = w;
        self.height = h;
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
    use crate::cell::{blocks, Rgb};

    const RED: Option<Rgb> = Some(Rgb { r: 205, g: 0, b: 0 });
    const BLUE: Option<Rgb> = Some(Rgb { r: 0, g: 0, b: 238 });

    #[test]
    fn test_new_canvas_is_empty() {
        let canvas = Canvas::new();
        assert_eq!(canvas.width, DEFAULT_WIDTH);
        assert_eq!(canvas.height, DEFAULT_HEIGHT);
        for y in 0..canvas.height {
            for x in 0..canvas.width {
                assert_eq!(canvas.get(x, y), Some(Cell::default()));
            }
        }
    }

    #[test]
    fn test_new_with_size() {
        let canvas = Canvas::new_with_size(16, 64);
        assert_eq!(canvas.width, 16);
        assert_eq!(canvas.height, 64);
        assert_eq!(canvas.get(15, 63), Some(Cell::default()));
        assert_eq!(canvas.get(16, 0), None);
    }

    #[test]
    fn test_clamp_dimensions() {
        let small = Canvas::new_with_size(2, 2);
        assert_eq!(small.width, MIN_DIMENSION);
        assert_eq!(small.height, MIN_DIMENSION);

        let big = Canvas::new_with_size(999, 999);
        assert_eq!(big.width, MAX_DIMENSION);
        assert_eq!(big.height, MAX_DIMENSION);
    }

    #[test]
    fn test_get_set() {
        let mut canvas = Canvas::new();
        let cell = Cell {
            ch: blocks::FULL,
            fg: RED,
            bg: BLUE,
        };
        canvas.set(5, 10, cell);
        assert_eq!(canvas.get(5, 10), Some(cell));
    }

    #[test]
    fn test_out_of_bounds_get() {
        let canvas = Canvas::new();
        assert_eq!(canvas.get(DEFAULT_WIDTH, 0), None);
        assert_eq!(canvas.get(0, DEFAULT_HEIGHT), None);
        assert_eq!(canvas.get(100, 100), None);
    }

    #[test]
    fn test_out_of_bounds_set() {
        let mut canvas = Canvas::new();
        let cell = Cell {
            ch: blocks::FULL,
            fg: RED,
            bg: None,
        };
        canvas.set(DEFAULT_WIDTH, 0, cell); // Should not panic
        canvas.set(0, DEFAULT_HEIGHT, cell); // Should not panic
    }

    #[test]
    fn test_clear() {
        let mut canvas = Canvas::new();
        let cell = Cell {
            ch: blocks::FULL,
            fg: RED,
            bg: BLUE,
        };
        canvas.set(0, 0, cell);
        canvas.set(31, 31, cell);
        canvas.clear();
        assert_eq!(canvas.get(0, 0), Some(Cell::default()));
        assert_eq!(canvas.get(31, 31), Some(Cell::default()));
    }

    #[test]
    fn test_resize_grow() {
        let mut canvas = Canvas::new_with_size(16, 16);
        let cell = Cell {
            ch: blocks::FULL,
            fg: RED,
            bg: None,
        };
        canvas.set(5, 5, cell);
        canvas.resize(32, 32);
        assert_eq!(canvas.width, 32);
        assert_eq!(canvas.height, 32);
        assert_eq!(canvas.get(5, 5), Some(cell));
        assert_eq!(canvas.get(20, 20), Some(Cell::default()));
    }

    #[test]
    fn test_resize_shrink() {
        let mut canvas = Canvas::new_with_size(32, 32);
        let cell = Cell {
            ch: blocks::FULL,
            fg: RED,
            bg: None,
        };
        canvas.set(5, 5, cell);
        canvas.set(20, 20, cell);
        canvas.resize(16, 16);
        assert_eq!(canvas.width, 16);
        assert_eq!(canvas.height, 16);
        assert_eq!(canvas.get(5, 5), Some(cell));
        assert_eq!(canvas.get(20, 20), None); // Now out of bounds
    }
}
