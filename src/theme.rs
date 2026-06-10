use ratatui::style::Color;

pub struct Theme {
    pub name: &'static str,
    pub border_accent: Color,
    pub header_bg: Color,
    pub highlight: Color,
    pub accent: Color,
    pub dim: Color,
    pub separator: Color,
    pub panel_bg: Color,
    pub grid_even: Color,
    pub grid_odd: Color,
    pub msg_success: Color,
    pub msg_warning: Color,
    pub msg_error: Color,
}

pub const THEMES: [Theme; 3] = [WARM, NEON, DARK];

pub const WARM: Theme = Theme {
    name: "Warm",
    border_accent: Color::Indexed(130),
    header_bg: Color::Indexed(130),
    highlight: Color::Indexed(220),
    accent: Color::Indexed(214),
    dim: Color::Indexed(243),
    separator: Color::Indexed(239),
    panel_bg: Color::Indexed(235),
    grid_even: Color::Indexed(235),
    grid_odd: Color::Indexed(234),
    msg_success: Color::Indexed(34),
    msg_warning: Color::Indexed(178),
    msg_error: Color::Indexed(160),
};

pub const NEON: Theme = Theme {
    name: "Neon",
    border_accent: Color::Indexed(201),
    header_bg: Color::Indexed(55),
    highlight: Color::Indexed(46),
    accent: Color::Indexed(51),
    dim: Color::Indexed(245),
    separator: Color::Indexed(240),
    panel_bg: Color::Indexed(233),
    grid_even: Color::Indexed(234),
    grid_odd: Color::Indexed(233),
    msg_success: Color::Indexed(46),
    msg_warning: Color::Indexed(226),
    msg_error: Color::Indexed(196),
};

pub const DARK: Theme = Theme {
    name: "Dark",
    border_accent: Color::Indexed(245),
    header_bg: Color::Indexed(236),
    highlight: Color::Indexed(255),
    accent: Color::Indexed(252),
    dim: Color::Indexed(241),
    separator: Color::Indexed(237),
    panel_bg: Color::Indexed(234),
    grid_even: Color::Indexed(236),
    grid_odd: Color::Indexed(235),
    msg_success: Color::Indexed(35),
    msg_warning: Color::Indexed(172),
    msg_error: Color::Indexed(124),
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_themes_count() {
        assert_eq!(THEMES.len(), 3);
    }

    #[test]
    fn test_theme_names() {
        assert_eq!(THEMES[0].name, "Warm");
        assert_eq!(THEMES[1].name, "Neon");
        assert_eq!(THEMES[2].name, "Dark");
    }

    #[test]
    fn test_warm_matches_legacy_constants() {
        assert_eq!(WARM.border_accent, Color::Indexed(130));
        assert_eq!(WARM.header_bg, Color::Indexed(130));
        assert_eq!(WARM.highlight, Color::Indexed(220));
        assert_eq!(WARM.accent, Color::Indexed(214));
        assert_eq!(WARM.dim, Color::Indexed(243));
        assert_eq!(WARM.separator, Color::Indexed(239));
        assert_eq!(WARM.panel_bg, Color::Indexed(235));
        assert_eq!(WARM.grid_even, Color::Indexed(235));
        assert_eq!(WARM.grid_odd, Color::Indexed(234));
    }

    #[test]
    fn test_theme_message_colors() {
        for theme in &THEMES {
            // All message colors must be Color::Indexed (not Reset or other variants)
            assert!(matches!(theme.msg_success, Color::Indexed(_)), "{} msg_success", theme.name);
            assert!(matches!(theme.msg_warning, Color::Indexed(_)), "{} msg_warning", theme.name);
            assert!(matches!(theme.msg_error, Color::Indexed(_)), "{} msg_error", theme.name);
        }
    }
}
