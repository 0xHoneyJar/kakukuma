use ratatui::style::Color;
use serde::Serialize;

/// Block element constants (U+2580–259F) for readability.
pub mod blocks {
    // Original 5
    pub const FULL: char       = '\u{2588}'; // █
    pub const UPPER_HALF: char = '\u{2580}'; // ▀
    pub const LOWER_HALF: char = '\u{2584}'; // ▄
    pub const LEFT_HALF: char  = '\u{258C}'; // ▌
    pub const RIGHT_HALF: char = '\u{2590}'; // ▐

    // Extended — lower fractional fills
    pub const LOWER_1_8: char = '\u{2581}'; // ▁
    pub const LOWER_1_4: char = '\u{2582}'; // ▂
    pub const LOWER_3_8: char = '\u{2583}'; // ▃
    pub const LOWER_5_8: char = '\u{2585}'; // ▅
    pub const LOWER_3_4: char = '\u{2586}'; // ▆
    pub const LOWER_7_8: char = '\u{2587}'; // ▇

    // Extended — left fractional fills
    pub const LEFT_7_8: char = '\u{2589}'; // ▉
    pub const LEFT_3_4: char = '\u{258A}'; // ▊
    pub const LEFT_5_8: char = '\u{258B}'; // ▋
    pub const LEFT_3_8: char = '\u{258D}'; // ▍
    pub const LEFT_1_4: char = '\u{258E}'; // ▎
    pub const LEFT_1_8: char = '\u{258F}'; // ▏

    // Shade patterns
    pub const SHADE_LIGHT: char  = '\u{2591}'; // ░
    pub const SHADE_MEDIUM: char = '\u{2592}'; // ▒
    pub const SHADE_DARK: char   = '\u{2593}'; // ▓

    /// Primary block cycle (B key): the original 5.
    pub const PRIMARY: [char; 5] = [FULL, UPPER_HALF, LOWER_HALF, LEFT_HALF, RIGHT_HALF];

    /// Shade block cycle (G key): stipple/texture patterns.
    pub const SHADES: [char; 3] = [SHADE_LIGHT, SHADE_MEDIUM, SHADE_DARK];

    /// Vertical fractional fills (bottom-up).
    pub const VERTICAL_FILLS: [char; 6] = [
        LOWER_1_8, LOWER_1_4, LOWER_3_8, LOWER_5_8, LOWER_3_4, LOWER_7_8,
    ];

    /// Horizontal fractional fills (left, decreasing).
    pub const HORIZONTAL_FILLS: [char; 6] = [
        LEFT_7_8, LEFT_3_4, LEFT_5_8, LEFT_3_8, LEFT_1_4, LEFT_1_8,
    ];

    /// All blocks in picker order (4 categories, 20 total).
    pub const ALL: [char; 20] = [
        FULL, UPPER_HALF, LOWER_HALF, LEFT_HALF, RIGHT_HALF,
        SHADE_LIGHT, SHADE_MEDIUM, SHADE_DARK,
        LOWER_1_8, LOWER_1_4, LOWER_3_8, LOWER_5_8, LOWER_3_4, LOWER_7_8,
        LEFT_7_8, LEFT_3_4, LEFT_5_8, LEFT_3_8, LEFT_1_4, LEFT_1_8,
    ];

    /// Category sizes for the block picker (Primary=5, Shades=3, Vert=6, Horiz=6).
    pub const CATEGORY_SIZES: [usize; 4] = [5, 3, 6, 6];
}

/// Classification helpers for rendering.
pub fn is_vertical_half(ch: char) -> bool {
    ch == blocks::UPPER_HALF || ch == blocks::LOWER_HALF
}

pub fn is_horizontal_half(ch: char) -> bool {
    ch == blocks::LEFT_HALF || ch == blocks::RIGHT_HALF
}

pub fn is_half_block(ch: char) -> bool {
    is_vertical_half(ch) || is_horizontal_half(ch)
}

/// Result of resolving a half-block cell's transparency.
/// `fg` and `bg` are `None` when that half is transparent.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ResolvedHalfBlock {
    pub ch: char,
    pub fg: Option<Rgb>,
    pub bg: Option<Rgb>,
}

/// Resolve a half-block cell into its canonical (char, fg, bg) representation,
/// flipping the character when one half is transparent.
/// Returns None if the cell is not a half-block character.
pub fn resolve_half_block(cell: &Cell) -> Option<ResolvedHalfBlock> {
    if !is_half_block(cell.ch) {
        return None;
    }

    // Normalize all four half-block characters to a canonical (primary, secondary)
    // pair, then resolve transparency uniformly.
    //
    // UPPER_HALF: fg=top, bg=bottom  -> normal='▀', flipped='▄'
    // LOWER_HALF: fg=bottom, bg=top  -> normal='▀', flipped='▄' (non-canonical storage)
    // LEFT_HALF:  fg=left, bg=right  -> normal='▌', flipped='▐'
    // RIGHT_HALF: fg=right, bg=left  -> normal='▌', flipped='▐' (non-canonical storage)
    let (primary, secondary, normal_ch, flipped_ch) = match cell.ch {
        blocks::UPPER_HALF => (cell.fg, cell.bg, blocks::UPPER_HALF, blocks::LOWER_HALF),
        blocks::LOWER_HALF => (cell.bg, cell.fg, blocks::UPPER_HALF, blocks::LOWER_HALF),
        blocks::LEFT_HALF  => (cell.fg, cell.bg, blocks::LEFT_HALF,  blocks::RIGHT_HALF),
        blocks::RIGHT_HALF => (cell.bg, cell.fg, blocks::LEFT_HALF,  blocks::RIGHT_HALF),
        _ => unreachable!(),
    };

    let resolved = match (primary, secondary) {
        (None, None)       => ResolvedHalfBlock { ch: ' ',         fg: None,      bg: None },
        (Some(_), None)    => ResolvedHalfBlock { ch: normal_ch,   fg: primary,   bg: None },
        (None, Some(_))    => ResolvedHalfBlock { ch: flipped_ch,  fg: secondary, bg: None },
        (Some(_), Some(_)) => ResolvedHalfBlock { ch: normal_ch,   fg: primary,   bg: secondary },
    };

    Some(resolved)
}

/// Cycle to next block in the primary set (B key).
pub fn next_primary(ch: char) -> char {
    let idx = blocks::PRIMARY.iter().position(|&c| c == ch);
    match idx {
        Some(i) => blocks::PRIMARY[(i + 1) % blocks::PRIMARY.len()],
        None => blocks::PRIMARY[0],
    }
}

/// Cycle to next shade block (G key): ░ → ▒ → ▓ → ░.
pub fn next_shade(ch: char) -> char {
    let idx = blocks::SHADES.iter().position(|&c| c == ch);
    match idx {
        Some(i) => blocks::SHADES[(i + 1) % blocks::SHADES.len()],
        None => blocks::SHADES[0],
    }
}

/// Parse a hex color string into an Rgb value.
/// Accepts "#RRGGBB", "RRGGBB", case-insensitive.
pub fn parse_hex_color(input: &str) -> Option<Rgb> {
    let hex = input.strip_prefix('#').unwrap_or(input);
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Rgb::new(r, g, b))
}

/// Convert a legacy BlockChar name to a char.
fn legacy_block_to_char(name: &str) -> char {
    match name {
        "Empty" => ' ',
        "Full" => blocks::FULL,
        "UpperHalf" => blocks::UPPER_HALF,
        "LowerHalf" => blocks::LOWER_HALF,
        "LeftHalf" => blocks::LEFT_HALF,
        "RightHalf" => blocks::RIGHT_HALF,
        _ => blocks::FULL, // Unknown → Full
    }
}

/// True-color RGB value.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const WHITE: Rgb = Rgb { r: 229, g: 229, b: 229 };
    #[allow(dead_code)] // Used in tests and palette defaults
    pub const BLACK: Rgb = Rgb { r: 0, g: 0, b: 0 };

    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Rgb { r, g, b }
    }

    pub fn to_ratatui(self) -> Color {
        Color::Indexed(nearest_256(&self))
    }

    /// Human-readable name. Returns hex string like "#FF0000".
    pub fn name(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }
}

impl Serialize for Rgb {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeTuple;
        let mut tup = serializer.serialize_tuple(3)?;
        tup.serialize_element(&self.r)?;
        tup.serialize_element(&self.g)?;
        tup.serialize_element(&self.b)?;
        tup.end()
    }
}

impl<'de> serde::Deserialize<'de> for Rgb {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de;

        struct RgbVisitor;

        impl<'de> de::Visitor<'de> for RgbVisitor {
            type Value = Rgb;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("an [r,g,b] array or a u8 color index")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Rgb, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let r = seq.next_element::<u8>()?.ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let g = seq.next_element::<u8>()?.ok_or_else(|| de::Error::invalid_length(1, &self))?;
                let b = seq.next_element::<u8>()?.ok_or_else(|| de::Error::invalid_length(2, &self))?;
                Ok(Rgb { r, g, b })
            }

            // Backward compat: accept a u8 color index and convert to RGB
            fn visit_u64<E>(self, value: u64) -> Result<Rgb, E>
            where
                E: de::Error,
            {
                if value > 255 {
                    Err(E::custom(format!("color index {} out of range 0–255", value)))
                } else {
                    Ok(color256_to_rgb(value as u8))
                }
            }

            // Backward compat: accept legacy ANSI color name strings
            fn visit_str<E>(self, value: &str) -> Result<Rgb, E>
            where
                E: de::Error,
            {
                let idx = match value {
                    "Black" => 0,
                    "Red" => 1,
                    "Green" => 2,
                    "Yellow" => 3,
                    "Blue" => 4,
                    "Magenta" => 5,
                    "Cyan" => 6,
                    "White" => 7,
                    "BrightBlack" => 8,
                    "BrightRed" => 9,
                    "BrightGreen" => 10,
                    "BrightYellow" => 11,
                    "BrightBlue" => 12,
                    "BrightMagenta" => 13,
                    "BrightCyan" => 14,
                    "BrightWhite" => 15,
                    _ => return Err(E::custom(format!("unknown color name: {}", value))),
                };
                Ok(color256_to_rgb(idx))
            }
        }

        deserializer.deserialize_any(RgbVisitor)
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

/// Convert a xterm-256 color index to an Rgb value.
pub fn color256_to_rgb(idx: u8) -> Rgb {
    let (r, g, b) = match idx {
        0..=15 => ANSI_16_RGB[idx as usize],
        16..=231 => {
            let i = idx - 16;
            let r = i / 36;
            let g = (i % 36) / 6;
            let b = i % 6;
            let to_val = |v: u8| if v == 0 { 0 } else { 55 + 40 * v };
            (to_val(r), to_val(g), to_val(b))
        }
        232..=255 => {
            let gray = 8 + 10 * (idx - 232);
            (gray, gray, gray)
        }
    };
    Rgb { r, g, b }
}

/// Find the nearest xterm-256 color index for an Rgb value (Euclidean distance).
pub fn nearest_256(color: &Rgb) -> u8 {
    let mut best_idx: u8 = 0;
    let mut best_dist = u32::MAX;

    for i in 0u16..=255 {
        let c = color256_to_rgb(i as u8);
        let dr = color.r as i32 - c.r as i32;
        let dg = color.g as i32 - c.g as i32;
        let db = color.b as i32 - c.b as i32;
        let dist = (dr * dr + dg * dg + db * db) as u32;
        if dist < best_dist {
            best_dist = dist;
            best_idx = i as u8;
        }
    }

    best_idx
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Cell {
    pub ch: char,
    pub fg: Option<Rgb>,
    pub bg: Option<Rgb>,
}

impl Cell {
    /// Canonical empty cell: space with no colors (transparent).
    pub fn empty() -> Self {
        Cell { ch: ' ', fg: None, bg: None }
    }

    pub fn is_empty(&self) -> bool {
        self.ch == ' '
    }
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            ch: ' ',
            fg: Some(Rgb::WHITE),
            bg: None,
        }
    }
}

impl Serialize for Cell {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("Cell", 3)?;
        s.serialize_field("ch", &self.ch)?;
        s.serialize_field("fg", &self.fg)?;
        s.serialize_field("bg", &self.bg)?;
        s.end()
    }
}

impl<'de> serde::Deserialize<'de> for Cell {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de;

        #[derive(serde::Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Ch,
            Block,
            Fg,
            Bg,
        }

        struct CellVisitor;

        impl<'de> de::Visitor<'de> for CellVisitor {
            type Value = Cell;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a Cell with 'ch' or legacy 'block' field, and fg/bg as [r,g,b], index, or null")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Cell, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut ch: Option<char> = None;
                let mut block: Option<String> = None;
                let mut fg: Option<Option<Rgb>> = None;
                let mut bg: Option<Option<Rgb>> = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Ch => { ch = Some(map.next_value()?); }
                        Field::Block => { block = Some(map.next_value()?); }
                        Field::Fg => { fg = Some(map.next_value()?); }
                        Field::Bg => { bg = Some(map.next_value()?); }
                    }
                }

                let resolved_ch = if let Some(c) = ch {
                    c
                } else if let Some(b) = block {
                    legacy_block_to_char(&b)
                } else {
                    return Err(de::Error::missing_field("ch"));
                };

                Ok(Cell {
                    ch: resolved_ch,
                    fg: fg.unwrap_or(Some(Rgb::WHITE)),
                    bg: bg.unwrap_or(None),
                })
            }
        }

        deserializer.deserialize_struct("Cell", &["ch", "block", "fg", "bg"], CellVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color256_to_rgb_standard() {
        let c = color256_to_rgb(0);
        assert_eq!((c.r, c.g, c.b), (0, 0, 0));
        let c = color256_to_rgb(7);
        assert_eq!((c.r, c.g, c.b), (229, 229, 229));
        let c = color256_to_rgb(9);
        assert_eq!((c.r, c.g, c.b), (255, 0, 0));
    }

    #[test]
    fn test_color256_to_rgb_cube() {
        let c = color256_to_rgb(16);
        assert_eq!((c.r, c.g, c.b), (0, 0, 0));
        let c = color256_to_rgb(196);
        assert_eq!((c.r, c.g, c.b), (255, 0, 0));
        let c = color256_to_rgb(21);
        assert_eq!((c.r, c.g, c.b), (0, 0, 255));
        let c = color256_to_rgb(46);
        assert_eq!((c.r, c.g, c.b), (0, 255, 0));
    }

    #[test]
    fn test_color256_to_rgb_grayscale() {
        let c = color256_to_rgb(232);
        assert_eq!((c.r, c.g, c.b), (8, 8, 8));
        let c = color256_to_rgb(255);
        assert_eq!((c.r, c.g, c.b), (238, 238, 238));
    }

    #[test]
    fn test_rgb_name() {
        assert_eq!(Rgb::new(255, 0, 0).name(), "#FF0000");
        assert_eq!(Rgb::BLACK.name(), "#000000");
        assert_eq!(Rgb::WHITE.name(), "#E5E5E5");
    }

    #[test]
    fn test_nearest_256_pure_red() {
        let idx = nearest_256(&Rgb::new(255, 0, 0));
        assert!(idx == 196 || idx == 9, "Got {}", idx);
    }

    #[test]
    fn test_nearest_256_black() {
        assert_eq!(nearest_256(&Rgb::BLACK), 0);
    }

    #[test]
    fn test_nearest_256_white() {
        let idx = nearest_256(&Rgb::new(255, 255, 255));
        assert!(idx == 15 || idx == 231, "Got {}", idx);
    }

    #[test]
    fn test_serialize_rgb() {
        let c = Rgb::new(255, 128, 0);
        let json = serde_json::to_string(&c).unwrap();
        assert_eq!(json, "[255,128,0]");
    }

    #[test]
    fn test_deserialize_rgb_array() {
        let c: Rgb = serde_json::from_str("[255,128,0]").unwrap();
        assert_eq!(c, Rgb::new(255, 128, 0));
    }

    #[test]
    fn test_deserialize_rgb_from_u8_index() {
        let c: Rgb = serde_json::from_str("196").unwrap();
        assert_eq!(c, color256_to_rgb(196));
    }

    #[test]
    fn test_deserialize_rgb_from_legacy_name() {
        let c: Rgb = serde_json::from_str("\"Red\"").unwrap();
        assert_eq!(c, color256_to_rgb(1));
        let c: Rgb = serde_json::from_str("\"BrightCyan\"").unwrap();
        assert_eq!(c, color256_to_rgb(14));
    }

    #[test]
    fn test_cell_roundtrip() {
        let cell = Cell {
            ch: blocks::FULL,
            fg: Some(Rgb::new(255, 0, 0)),
            bg: None,
        };
        let json = serde_json::to_string(&cell).unwrap();
        let loaded: Cell = serde_json::from_str(&json).unwrap();
        assert_eq!(cell, loaded);
    }

    #[test]
    fn test_cell_roundtrip_with_both_colors() {
        let cell = Cell {
            ch: blocks::UPPER_HALF,
            fg: Some(Rgb::new(255, 0, 0)),
            bg: Some(Rgb::new(0, 0, 255)),
        };
        let json = serde_json::to_string(&cell).unwrap();
        let loaded: Cell = serde_json::from_str(&json).unwrap();
        assert_eq!(cell, loaded);
    }

    #[test]
    fn test_cell_legacy_v1_roundtrip() {
        // Legacy v1 cell JSON with "block" field and string color names
        let json = r#"{"block":"Full","fg":"Red","bg":"Black"}"#;
        let cell: Cell = serde_json::from_str(json).unwrap();
        assert_eq!(cell.fg, Some(color256_to_rgb(1)));
        assert_eq!(cell.bg, Some(color256_to_rgb(0)));
        assert_eq!(cell.ch, blocks::FULL);
    }

    #[test]
    fn test_cell_v4_format() {
        // v4 format: ch + u8 color indices
        let json = r#"{"ch":"█","fg":1,"bg":0}"#;
        let cell: Cell = serde_json::from_str(json).unwrap();
        assert_eq!(cell.ch, blocks::FULL);
        assert_eq!(cell.fg, Some(color256_to_rgb(1)));
        assert_eq!(cell.bg, Some(color256_to_rgb(0)));
    }

    #[test]
    fn test_cell_v5_format() {
        // v5 format: ch + [r,g,b] arrays or null
        let json = r#"{"ch":"█","fg":[255,0,0],"bg":null}"#;
        let cell: Cell = serde_json::from_str(json).unwrap();
        assert_eq!(cell.ch, blocks::FULL);
        assert_eq!(cell.fg, Some(Rgb::new(255, 0, 0)));
        assert_eq!(cell.bg, None);
    }

    #[test]
    fn test_cell_halfblock_roundtrip() {
        let cell = Cell {
            ch: blocks::UPPER_HALF,
            fg: Some(Rgb::new(255, 0, 0)),
            bg: None,
        };
        let json = serde_json::to_string(&cell).unwrap();
        let loaded: Cell = serde_json::from_str(&json).unwrap();
        assert_eq!(cell, loaded);
    }

    // --- Block cycling tests ---

    #[test]
    fn test_next_primary_cycle() {
        assert_eq!(next_primary(blocks::FULL), blocks::UPPER_HALF);
        assert_eq!(next_primary(blocks::UPPER_HALF), blocks::LOWER_HALF);
        assert_eq!(next_primary(blocks::LOWER_HALF), blocks::LEFT_HALF);
        assert_eq!(next_primary(blocks::LEFT_HALF), blocks::RIGHT_HALF);
        assert_eq!(next_primary(blocks::RIGHT_HALF), blocks::FULL);
        // Non-primary char → Full
        assert_eq!(next_primary(' '), blocks::FULL);
    }

    #[test]
    fn test_next_shade_cycle() {
        assert_eq!(next_shade(blocks::SHADE_LIGHT), blocks::SHADE_MEDIUM);
        assert_eq!(next_shade(blocks::SHADE_MEDIUM), blocks::SHADE_DARK);
        assert_eq!(next_shade(blocks::SHADE_DARK), blocks::SHADE_LIGHT); // wraps
    }

    #[test]
    fn test_next_shade_non_shade_input() {
        // Non-shade char → first shade (LIGHT)
        assert_eq!(next_shade(blocks::FULL), blocks::SHADE_LIGHT);
        assert_eq!(next_shade(' '), blocks::SHADE_LIGHT);
    }

    #[test]
    fn test_blocks_all_count() {
        assert_eq!(blocks::ALL.len(), 20);
    }

    #[test]
    fn test_blocks_all_unique() {
        let mut seen = std::collections::HashSet::new();
        for &ch in &blocks::ALL {
            assert!(seen.insert(ch), "Duplicate block character: {:?}", ch);
        }
    }

    #[test]
    fn test_category_sizes_sum() {
        let total: usize = blocks::CATEGORY_SIZES.iter().sum();
        assert_eq!(total, blocks::ALL.len());
    }

    #[test]
    fn test_classification_helpers() {
        assert!(is_vertical_half(blocks::UPPER_HALF));
        assert!(is_vertical_half(blocks::LOWER_HALF));
        assert!(!is_vertical_half(blocks::LEFT_HALF));
        assert!(!is_vertical_half(blocks::FULL));

        assert!(is_horizontal_half(blocks::LEFT_HALF));
        assert!(is_horizontal_half(blocks::RIGHT_HALF));
        assert!(!is_horizontal_half(blocks::UPPER_HALF));

        assert!(is_half_block(blocks::UPPER_HALF));
        assert!(is_half_block(blocks::LEFT_HALF));
        assert!(!is_half_block(blocks::FULL));
        assert!(!is_half_block(' '));
    }

    #[test]
    fn test_cell_is_empty() {
        assert!(Cell::default().is_empty());
        assert!(!Cell { ch: blocks::FULL, fg: Some(Rgb::new(205, 0, 0)), bg: None }.is_empty());
    }

    // --- resolve_half_block tests ---

    const RED: Rgb = Rgb { r: 205, g: 0, b: 0 };
    const BLUE: Rgb = Rgb { r: 0, g: 0, b: 238 };

    #[test]
    fn resolve_non_half_block_returns_none() {
        let cell = Cell { ch: blocks::FULL, fg: Some(RED), bg: None };
        assert!(resolve_half_block(&cell).is_none());
        let cell = Cell { ch: ' ', fg: None, bg: None };
        assert!(resolve_half_block(&cell).is_none());
    }

    #[test]
    fn resolve_upper_half_both_opaque() {
        let cell = Cell { ch: blocks::UPPER_HALF, fg: Some(RED), bg: Some(BLUE) };
        let r = resolve_half_block(&cell).unwrap();
        assert_eq!(r.ch, blocks::UPPER_HALF);
        assert_eq!(r.fg, Some(RED));
        assert_eq!(r.bg, Some(BLUE));
    }

    #[test]
    fn resolve_upper_half_top_transparent_flips() {
        let cell = Cell { ch: blocks::UPPER_HALF, fg: None, bg: Some(BLUE) };
        let r = resolve_half_block(&cell).unwrap();
        assert_eq!(r.ch, blocks::LOWER_HALF);
        assert_eq!(r.fg, Some(BLUE));
        assert_eq!(r.bg, None);
    }

    #[test]
    fn resolve_upper_half_bottom_transparent() {
        let cell = Cell { ch: blocks::UPPER_HALF, fg: Some(RED), bg: None };
        let r = resolve_half_block(&cell).unwrap();
        assert_eq!(r.ch, blocks::UPPER_HALF);
        assert_eq!(r.fg, Some(RED));
        assert_eq!(r.bg, None);
    }

    #[test]
    fn resolve_upper_half_both_transparent() {
        let cell = Cell { ch: blocks::UPPER_HALF, fg: None, bg: None };
        let r = resolve_half_block(&cell).unwrap();
        assert_eq!(r.ch, ' ');
        assert_eq!(r.fg, None);
        assert_eq!(r.bg, None);
    }

    #[test]
    fn resolve_lower_half_both_opaque() {
        // LOWER_HALF: fg=bottom, bg=top — normalizes to UPPER_HALF with top=bg, bottom=fg
        let cell = Cell { ch: blocks::LOWER_HALF, fg: Some(RED), bg: Some(BLUE) };
        let r = resolve_half_block(&cell).unwrap();
        assert_eq!(r.ch, blocks::UPPER_HALF);
        assert_eq!(r.fg, Some(BLUE)); // top (bg) becomes primary
        assert_eq!(r.bg, Some(RED));  // bottom (fg) becomes secondary
    }

    #[test]
    fn resolve_lower_half_top_transparent_flips() {
        // bg=top=None, fg=bottom=RED -> flipped to LOWER_HALF with fg=RED
        let cell = Cell { ch: blocks::LOWER_HALF, fg: Some(RED), bg: None };
        let r = resolve_half_block(&cell).unwrap();
        assert_eq!(r.ch, blocks::LOWER_HALF);
        assert_eq!(r.fg, Some(RED));
        assert_eq!(r.bg, None);
    }

    #[test]
    fn resolve_left_half_both_opaque() {
        let cell = Cell { ch: blocks::LEFT_HALF, fg: Some(RED), bg: Some(BLUE) };
        let r = resolve_half_block(&cell).unwrap();
        assert_eq!(r.ch, blocks::LEFT_HALF);
        assert_eq!(r.fg, Some(RED));
        assert_eq!(r.bg, Some(BLUE));
    }

    #[test]
    fn resolve_left_half_left_transparent_flips() {
        let cell = Cell { ch: blocks::LEFT_HALF, fg: None, bg: Some(BLUE) };
        let r = resolve_half_block(&cell).unwrap();
        assert_eq!(r.ch, blocks::RIGHT_HALF);
        assert_eq!(r.fg, Some(BLUE));
        assert_eq!(r.bg, None);
    }

    #[test]
    fn resolve_right_half_both_opaque() {
        // RIGHT_HALF: fg=right, bg=left — normalizes to LEFT_HALF with left=bg, right=fg
        let cell = Cell { ch: blocks::RIGHT_HALF, fg: Some(RED), bg: Some(BLUE) };
        let r = resolve_half_block(&cell).unwrap();
        assert_eq!(r.ch, blocks::LEFT_HALF);
        assert_eq!(r.fg, Some(BLUE)); // left (bg) becomes primary
        assert_eq!(r.bg, Some(RED));  // right (fg) becomes secondary
    }

    #[test]
    fn resolve_right_half_left_transparent_flips() {
        // bg=left=None, fg=right=RED -> flipped to RIGHT_HALF with fg=RED
        let cell = Cell { ch: blocks::RIGHT_HALF, fg: Some(RED), bg: None };
        let r = resolve_half_block(&cell).unwrap();
        assert_eq!(r.ch, blocks::RIGHT_HALF);
        assert_eq!(r.fg, Some(RED));
        assert_eq!(r.bg, None);
    }

    // --- parse_hex_color tests ---

    #[test]
    fn parse_hex_with_hash() {
        assert_eq!(parse_hex_color("#FF0000"), Some(Rgb::new(255, 0, 0)));
    }

    #[test]
    fn parse_hex_without_hash() {
        assert_eq!(parse_hex_color("00FF00"), Some(Rgb::new(0, 255, 0)));
    }

    #[test]
    fn parse_hex_lowercase() {
        assert_eq!(parse_hex_color("#ff8700"), Some(Rgb::new(255, 135, 0)));
    }

    #[test]
    fn parse_hex_mixed_case() {
        assert_eq!(parse_hex_color("#aAbBcC"), Some(Rgb::new(170, 187, 204)));
    }

    #[test]
    fn parse_hex_too_short() {
        assert_eq!(parse_hex_color("#FFF"), None);
    }

    #[test]
    fn parse_hex_invalid_chars() {
        assert_eq!(parse_hex_color("#GGHHII"), None);
    }

    #[test]
    fn parse_hex_empty() {
        assert_eq!(parse_hex_color(""), None);
    }
}
