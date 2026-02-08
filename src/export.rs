use crate::canvas::{Canvas, CANVAS_HEIGHT, CANVAS_WIDTH};
use crate::cell::BlockChar;

/// Export canvas as plain Unicode (block characters only, no color).
/// Each cell is doubled for square pixels.
pub fn to_plain_text(canvas: &Canvas) -> String {
    let mut output = String::new();
    let mut last_non_empty_row = 0;

    // Find last row with content
    for y in 0..CANVAS_HEIGHT {
        for x in 0..CANVAS_WIDTH {
            if let Some(cell) = canvas.get(x, y) {
                if cell.block != BlockChar::Empty {
                    last_non_empty_row = y;
                    break;
                }
            }
        }
    }

    for y in 0..=last_non_empty_row {
        let mut row = String::new();
        for x in 0..CANVAS_WIDTH {
            if let Some(cell) = canvas.get(x, y) {
                let ch = cell.block.to_char();
                row.push(ch);
                row.push(ch);
            }
        }
        // Strip trailing spaces
        let trimmed = row.trim_end();
        output.push_str(trimmed);
        if y < last_non_empty_row {
            output.push('\n');
        }
    }

    output
}

/// Export canvas as ANSI art (Unicode blocks with 256-color escape codes).
/// Each cell is doubled for square pixels.
/// Uses 256-color escape codes: \x1b[38;5;{fg}m and \x1b[48;5;{bg}m
pub fn to_ansi(canvas: &Canvas) -> String {
    let mut output = String::new();
    let mut last_non_empty_row = 0;

    for y in 0..CANVAS_HEIGHT {
        for x in 0..CANVAS_WIDTH {
            if let Some(cell) = canvas.get(x, y) {
                if cell.block != BlockChar::Empty {
                    last_non_empty_row = y;
                    break;
                }
            }
        }
    }

    for y in 0..=last_non_empty_row {
        let mut prev_fg = None;
        let mut prev_bg = None;

        for x in 0..CANVAS_WIDTH {
            if let Some(cell) = canvas.get(x, y) {
                let ch = cell.block.to_char();

                // Only emit color codes when colors change
                let fg_changed = prev_fg != Some(cell.fg);
                let bg_changed = prev_bg != Some(cell.bg);

                if fg_changed || bg_changed {
                    // Use 256-color escape codes
                    if cell.bg.0 != 0 {
                        output.push_str(&format!(
                            "\x1b[38;5;{};48;5;{}m",
                            cell.fg.0, cell.bg.0
                        ));
                    } else {
                        output.push_str(&format!(
                            "\x1b[38;5;{}m",
                            cell.fg.0
                        ));
                    }
                    prev_fg = Some(cell.fg);
                    prev_bg = Some(cell.bg);
                }

                output.push(ch);
                output.push(ch);
            }
        }

        output.push_str("\x1b[0m"); // Reset at end of line
        if y < last_non_empty_row {
            output.push('\n');
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::{Cell, Color256};

    #[test]
    fn test_plain_text_empty() {
        let canvas = Canvas::new();
        let text = to_plain_text(&canvas);
        assert!(text.is_empty() || text.trim().is_empty());
    }

    #[test]
    fn test_plain_text_single_block() {
        let mut canvas = Canvas::new();
        canvas.set(
            0,
            0,
            Cell {
                block: BlockChar::Full,
                fg: Color256(1),
                bg: Color256::BLACK,
            },
        );
        let text = to_plain_text(&canvas);
        assert_eq!(text, "\u{2588}\u{2588}");
    }

    #[test]
    fn test_plain_text_no_gaps() {
        let mut canvas = Canvas::new();
        for x in 0..3 {
            canvas.set(
                x,
                0,
                Cell {
                    block: BlockChar::Full,
                    fg: Color256::WHITE,
                    bg: Color256::BLACK,
                },
            );
        }
        let text = to_plain_text(&canvas);
        // Should be 6 block characters with no spaces between them
        assert_eq!(text, "\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}");
        assert!(!text.contains(' '));
    }

    #[test]
    fn test_ansi_has_256_color_codes() {
        let mut canvas = Canvas::new();
        canvas.set(
            0,
            0,
            Cell {
                block: BlockChar::Full,
                fg: Color256(1),
                bg: Color256::BLACK,
            },
        );
        let ansi = to_ansi(&canvas);
        assert!(ansi.contains("\x1b[38;5;1m"));
        assert!(ansi.contains("\x1b[0m"));
    }

    #[test]
    fn test_ansi_extended_color() {
        let mut canvas = Canvas::new();
        canvas.set(
            0,
            0,
            Cell {
                block: BlockChar::Full,
                fg: Color256(196),
                bg: Color256::BLACK,
            },
        );
        let ansi = to_ansi(&canvas);
        assert!(ansi.contains("\x1b[38;5;196m"));
    }

    #[test]
    fn test_ansi_with_bg_color() {
        let mut canvas = Canvas::new();
        canvas.set(
            0,
            0,
            Cell {
                block: BlockChar::Full,
                fg: Color256(7),
                bg: Color256(4),
            },
        );
        let ansi = to_ansi(&canvas);
        assert!(ansi.contains("\x1b[38;5;7;48;5;4m"));
    }
}
