use ratatui::style::Color;
use serde::Serialize;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub enum BlockChar {
    Empty,
    Full,
}

impl BlockChar {
    pub fn to_char(self) -> char {
        match self {
            BlockChar::Empty => ' ',
            BlockChar::Full => '\u{2588}',
        }
    }
}

// Custom Deserialize: map legacy half-block variants to Full
impl<'de> serde::Deserialize<'de> for BlockChar {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "Empty" => Ok(BlockChar::Empty),
            _ => Ok(BlockChar::Full), // Full, UpperHalf, LowerHalf, LeftHalf, RightHalf → Full
        }
    }
}

/// xterm-256 color index (0–255).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Color256(pub u8);

impl Color256 {
    pub const WHITE: Color256 = Color256(7);
    pub const BLACK: Color256 = Color256(0);

    pub fn to_ratatui(self) -> Color {
        Color::Indexed(self.0)
    }

    /// Convert index to approximate (R, G, B).
    pub fn to_rgb(self) -> (u8, u8, u8) {
        match self.0 {
            0..=15 => ANSI_16_RGB[self.0 as usize],
            16..=231 => {
                let idx = self.0 - 16;
                let r = idx / 36;
                let g = (idx % 36) / 6;
                let b = idx % 6;
                let to_val = |v: u8| if v == 0 { 0 } else { 55 + 40 * v };
                (to_val(r), to_val(g), to_val(b))
            }
            232..=255 => {
                let gray = 8 + 10 * (self.0 - 232);
                (gray, gray, gray)
            }
        }
    }

    /// Name for the first 16 standard colors; index string for others.
    pub fn name(self) -> String {
        match self.0 {
            0 => "Black".into(),
            1 => "Red".into(),
            2 => "Green".into(),
            3 => "Yellow".into(),
            4 => "Blue".into(),
            5 => "Magenta".into(),
            6 => "Cyan".into(),
            7 => "White".into(),
            8 => "BrightBlack".into(),
            9 => "BrightRed".into(),
            10 => "BrightGreen".into(),
            11 => "BrightYellow".into(),
            12 => "BrightBlue".into(),
            13 => "BrightMagenta".into(),
            14 => "BrightCyan".into(),
            15 => "BrightWhite".into(),
            n => format!("#{}", n),
        }
    }
}

/// Conventional RGB values for the 16 standard ANSI colors (indices 0–15).
pub const ANSI_16_RGB: [(u8, u8, u8); 16] = [
    (0, 0, 0),       // 0  Black
    (205, 0, 0),     // 1  Red
    (0, 205, 0),     // 2  Green
    (205, 205, 0),   // 3  Yellow
    (0, 0, 238),     // 4  Blue
    (205, 0, 205),   // 5  Magenta
    (0, 205, 205),   // 6  Cyan
    (229, 229, 229), // 7  White
    (127, 127, 127), // 8  BrightBlack
    (255, 0, 0),     // 9  BrightRed
    (0, 255, 0),     // 10 BrightGreen
    (255, 255, 0),   // 11 BrightYellow
    (92, 92, 255),   // 12 BrightBlue
    (255, 0, 255),   // 13 BrightMagenta
    (0, 255, 255),   // 14 BrightCyan
    (255, 255, 255), // 15 BrightWhite
];

impl Serialize for Color256 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u8(self.0)
    }
}

impl<'de> serde::Deserialize<'de> for Color256 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de;

        struct Color256Visitor;

        impl<'de> de::Visitor<'de> for Color256Visitor {
            type Value = Color256;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a u8 color index or a legacy ANSI color name string")
            }

            fn visit_u64<E>(self, value: u64) -> Result<Color256, E>
            where
                E: de::Error,
            {
                if value > 255 {
                    Err(E::custom(format!("color index {} out of range 0–255", value)))
                } else {
                    Ok(Color256(value as u8))
                }
            }

            fn visit_str<E>(self, value: &str) -> Result<Color256, E>
            where
                E: de::Error,
            {
                match value {
                    "Black" => Ok(Color256(0)),
                    "Red" => Ok(Color256(1)),
                    "Green" => Ok(Color256(2)),
                    "Yellow" => Ok(Color256(3)),
                    "Blue" => Ok(Color256(4)),
                    "Magenta" => Ok(Color256(5)),
                    "Cyan" => Ok(Color256(6)),
                    "White" => Ok(Color256(7)),
                    "BrightBlack" => Ok(Color256(8)),
                    "BrightRed" => Ok(Color256(9)),
                    "BrightGreen" => Ok(Color256(10)),
                    "BrightYellow" => Ok(Color256(11)),
                    "BrightBlue" => Ok(Color256(12)),
                    "BrightMagenta" => Ok(Color256(13)),
                    "BrightCyan" => Ok(Color256(14)),
                    "BrightWhite" => Ok(Color256(15)),
                    _ => Err(E::custom(format!("unknown color name: {}", value))),
                }
            }
        }

        deserializer.deserialize_any(Color256Visitor)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, serde::Deserialize)]
pub struct Cell {
    pub block: BlockChar,
    pub fg: Color256,
    pub bg: Color256,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            block: BlockChar::Empty,
            fg: Color256::WHITE,
            bg: Color256::BLACK,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_rgb_standard() {
        assert_eq!(Color256(0).to_rgb(), (0, 0, 0));
        assert_eq!(Color256(7).to_rgb(), (229, 229, 229));
        assert_eq!(Color256(9).to_rgb(), (255, 0, 0));
    }

    #[test]
    fn test_to_rgb_cube() {
        // Index 16 = R:0 G:0 B:0 in the cube → (0,0,0)
        assert_eq!(Color256(16).to_rgb(), (0, 0, 0));
        // Index 196 = R:5 G:0 B:0 → (255,0,0)
        assert_eq!(Color256(196).to_rgb(), (255, 0, 0));
        // Index 21 = R:0 G:0 B:5 → (0,0,255)
        assert_eq!(Color256(21).to_rgb(), (0, 0, 255));
        // Index 46 = R:0 G:5 B:0 → (0,255,0)
        assert_eq!(Color256(46).to_rgb(), (0, 255, 0));
    }

    #[test]
    fn test_to_rgb_grayscale() {
        assert_eq!(Color256(232).to_rgb(), (8, 8, 8));
        assert_eq!(Color256(255).to_rgb(), (238, 238, 238));
    }

    #[test]
    fn test_name_standard() {
        assert_eq!(Color256(0).name(), "Black");
        assert_eq!(Color256(7).name(), "White");
        assert_eq!(Color256(15).name(), "BrightWhite");
    }

    #[test]
    fn test_name_extended() {
        assert_eq!(Color256(100).name(), "#100");
        assert_eq!(Color256(232).name(), "#232");
    }

    #[test]
    fn test_serialize_u8() {
        let c = Color256(42);
        let json = serde_json::to_string(&c).unwrap();
        assert_eq!(json, "42");
    }

    #[test]
    fn test_deserialize_u8() {
        let c: Color256 = serde_json::from_str("42").unwrap();
        assert_eq!(c, Color256(42));
    }

    #[test]
    fn test_deserialize_legacy_name() {
        let c: Color256 = serde_json::from_str("\"Red\"").unwrap();
        assert_eq!(c, Color256(1));
        let c: Color256 = serde_json::from_str("\"BrightCyan\"").unwrap();
        assert_eq!(c, Color256(14));
    }

    #[test]
    fn test_cell_roundtrip() {
        let cell = Cell {
            block: BlockChar::Full,
            fg: Color256(100),
            bg: Color256(0),
        };
        let json = serde_json::to_string(&cell).unwrap();
        let loaded: Cell = serde_json::from_str(&json).unwrap();
        assert_eq!(cell, loaded);
    }

    #[test]
    fn test_cell_legacy_roundtrip() {
        // Simulate a legacy v1 cell JSON with string color names
        let json = r#"{"block":"Full","fg":"Red","bg":"Black"}"#;
        let cell: Cell = serde_json::from_str(json).unwrap();
        assert_eq!(cell.fg, Color256(1));
        assert_eq!(cell.bg, Color256(0));
        assert_eq!(cell.block, BlockChar::Full);
    }
}
