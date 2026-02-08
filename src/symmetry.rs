use serde::{Deserialize, Serialize};

use crate::canvas::CANVAS_WIDTH;
use crate::canvas::CANVAS_HEIGHT;
use crate::history::CellMutation;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum SymmetryMode {
    Off,
    Horizontal,
    Vertical,
    Quad,
}

impl SymmetryMode {
    pub fn toggle_horizontal(self) -> SymmetryMode {
        match self {
            SymmetryMode::Off => SymmetryMode::Horizontal,
            SymmetryMode::Horizontal => SymmetryMode::Off,
            SymmetryMode::Vertical => SymmetryMode::Quad,
            SymmetryMode::Quad => SymmetryMode::Vertical,
        }
    }

    pub fn toggle_vertical(self) -> SymmetryMode {
        match self {
            SymmetryMode::Off => SymmetryMode::Vertical,
            SymmetryMode::Vertical => SymmetryMode::Off,
            SymmetryMode::Horizontal => SymmetryMode::Quad,
            SymmetryMode::Quad => SymmetryMode::Horizontal,
        }
    }

    pub fn has_horizontal(self) -> bool {
        matches!(self, SymmetryMode::Horizontal | SymmetryMode::Quad)
    }

    pub fn has_vertical(self) -> bool {
        matches!(self, SymmetryMode::Vertical | SymmetryMode::Quad)
    }

    pub fn label(self) -> &'static str {
        match self {
            SymmetryMode::Off => "Off",
            SymmetryMode::Horizontal => "Horiz",
            SymmetryMode::Vertical => "Vert",
            SymmetryMode::Quad => "Quad",
        }
    }
}

/// Given a list of mutations, produce mirrored copies based on symmetry mode.
/// Returns the original mutations plus any mirrored ones.
pub fn apply_symmetry(mutations: Vec<CellMutation>, mode: SymmetryMode) -> Vec<CellMutation> {
    if mode == SymmetryMode::Off {
        return mutations;
    }

    let mut result = Vec::with_capacity(mutations.len() * 4);

    for m in &mutations {
        result.push(m.clone());

        if mode.has_horizontal() {
            let mx = CANVAS_WIDTH - 1 - m.x;
            if mx != m.x {
                let mut mirrored = m.clone();
                mirrored.x = mx;
                result.push(mirrored);
            }
        }

        if mode.has_vertical() {
            let my = CANVAS_HEIGHT - 1 - m.y;
            if my != m.y {
                let mut mirrored = m.clone();
                mirrored.y = my;
                result.push(mirrored);
            }
        }

        if mode == SymmetryMode::Quad {
            let mx = CANVAS_WIDTH - 1 - m.x;
            let my = CANVAS_HEIGHT - 1 - m.y;
            if mx != m.x && my != m.y {
                let mut mirrored = m.clone();
                mirrored.x = mx;
                mirrored.y = my;
                result.push(mirrored);
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::{BlockChar, Cell, Color256};

    fn make_mutation(x: usize, y: usize) -> CellMutation {
        CellMutation {
            x,
            y,
            old: Cell::default(),
            new: Cell {
                block: BlockChar::Full,
                fg: Color256(1),
                bg: Color256::BLACK,
            },
        }
    }

    #[test]
    fn test_off_no_mirror() {
        let mutations = vec![make_mutation(5, 10)];
        let result = apply_symmetry(mutations, SymmetryMode::Off);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_horizontal_mirror() {
        let mutations = vec![make_mutation(5, 10)];
        let result = apply_symmetry(mutations, SymmetryMode::Horizontal);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].x, 5);
        assert_eq!(result[1].x, 26); // 31 - 5
    }

    #[test]
    fn test_vertical_mirror() {
        let mutations = vec![make_mutation(5, 10)];
        let result = apply_symmetry(mutations, SymmetryMode::Vertical);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].y, 10);
        assert_eq!(result[1].y, 21); // 31 - 10
    }

    #[test]
    fn test_quad_mirror() {
        let mutations = vec![make_mutation(5, 10)];
        let result = apply_symmetry(mutations, SymmetryMode::Quad);
        assert_eq!(result.len(), 4);
        assert_eq!((result[0].x, result[0].y), (5, 10));
        assert_eq!((result[1].x, result[1].y), (26, 10));
        assert_eq!((result[2].x, result[2].y), (5, 21));
        assert_eq!((result[3].x, result[3].y), (26, 21));
    }

    #[test]
    fn test_center_axis_no_duplicate() {
        // Point on the horizontal center axis (x=15, x mirrored = 16, not same)
        // Point exactly on center for odd: with 32 width, there's no exact center cell
        let mutations = vec![make_mutation(15, 10)];
        let result = apply_symmetry(mutations, SymmetryMode::Horizontal);
        assert_eq!(result.len(), 2);
        assert_eq!(result[1].x, 16); // 31 - 15
    }
}
