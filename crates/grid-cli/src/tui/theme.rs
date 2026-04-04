//! Ratatui theme system for TUI mode
//!
//! Maps the 12 CLI color themes to Ratatui `Color`/`Style` types,
//! complementing the existing `crate::ui::theme` (owo-colors) module.

use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders};

use crate::ui::theme::ThemeName;

/// TUI-specific theme with Ratatui colors
#[derive(Debug, Clone)]
pub struct TuiTheme {
    /// Primary accent color
    pub accent: Color,
    /// Dimmed accent (borders, inactive elements)
    pub accent_dim: Color,
    /// Lighter accent shade (glow / hover effect)
    pub accent_glow: Color,
    /// Text rendered on an accent-colored background
    pub accent_text: Color,
    /// Secondary accent (for gradient themes like Sunset)
    pub accent2: Option<Color>,

    // -- Semantic colors --
    pub success: Color,
    pub error: Color,
    pub warning: Color,
    pub info: Color,
    pub muted: Color,

    // -- Surface colors (4-layer depth system from GRID_UI_UX_DESIGN §3.1) --
    pub surface: Color,
    pub surface_1: Color,
    pub surface_2: Color,
    pub surface_3: Color,
    pub border: Color,

    // -- Text (3-layer) --
    pub text: Color,
    pub text_secondary: Color,
    pub text_faint: Color,

    // -- Markdown rendering (derived from accent) --
    pub md_heading: Color,
    pub md_code_fg: Color,
    pub md_code_bg: Color,
    pub md_bold: Color,
    pub md_bullet: Color,
    pub md_link: Color,

    // -- Diff & tool colors --
    pub diff_add_bg: Color,
    pub diff_del_bg: Color,

    /// Source theme name
    pub name: ThemeName,
}

impl TuiTheme {
    /// Create a TUI theme from a CLI theme name.
    pub fn from_cli_theme(name: ThemeName) -> Self {
        let (r, g, b) = match name {
            ThemeName::Cyan => (6, 182, 212),
            ThemeName::Sgcc => (0, 132, 61),
            ThemeName::Blue => (59, 130, 246),
            ThemeName::Indigo => (99, 102, 241),
            ThemeName::Violet => (139, 92, 246),
            ThemeName::Emerald => (16, 185, 129),
            ThemeName::Amber => (245, 158, 11),
            ThemeName::Coral => (249, 115, 22),
            ThemeName::Rose => (244, 63, 94),
            ThemeName::Teal => (20, 184, 166),
            ThemeName::Sunset => (236, 72, 153),
            ThemeName::Slate => (148, 163, 184),
        };

        let accent = Color::Rgb(r, g, b);
        let accent_dim = Color::Rgb(r / 2, g / 2, b / 2);
        let accent_glow = Color::Rgb(
            ((r as u16 + 255).min(510) / 2) as u8,
            ((g as u16 + 255).min(510) / 2) as u8,
            ((b as u16 + 255).min(510) / 2) as u8,
        );

        let accent2 = match name {
            ThemeName::Sunset => Some(Color::Rgb(245, 158, 11)),
            _ => None,
        };

        Self {
            accent,
            accent_dim,
            accent_glow,
            accent_text: Color::Rgb(r, g, b),
            accent2,
            success: Color::Rgb(34, 197, 94),
            error: Color::Rgb(239, 68, 68),
            warning: Color::Rgb(245, 158, 11),
            info: Color::Rgb(56, 189, 248),
            muted: Color::Rgb(100, 116, 139),
            surface: Color::Rgb(10, 10, 15),
            surface_1: Color::Rgb(17, 17, 24),
            surface_2: Color::Rgb(26, 26, 36),
            surface_3: Color::Rgb(36, 36, 48),
            border: Color::Rgb(38, 38, 46),
            text: Color::Rgb(237, 237, 239),
            text_secondary: Color::Rgb(138, 143, 152),
            text_faint: Color::Rgb(78, 81, 88),
            md_heading: accent_glow,
            md_code_fg: Color::Rgb(150, 190, 160),
            md_code_bg: Color::Rgb(17, 17, 24),
            md_bold: Color::Rgb(237, 237, 239),
            md_bullet: Color::Rgb(138, 143, 152),
            md_link: Color::Rgb(74, 158, 255),
            diff_add_bg: Color::Rgb(10, 35, 25),
            diff_del_bg: Color::Rgb(40, 15, 15),
            name,
        }
    }

    // -- Convenience style constructors --

    /// Style for the active tab label.
    pub fn tab_active(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    }

    /// Style for inactive tab labels.
    pub fn tab_inactive(&self) -> Style {
        Style::default().fg(self.muted)
    }

    /// Style for block titles.
    pub fn block_title(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for block borders.
    pub fn block_border(&self) -> Style {
        Style::default().fg(self.border)
    }

    /// Style for active/focused block borders.
    pub fn block_border_active(&self) -> Style {
        Style::default().fg(self.accent)
    }

    /// Style for normal body text.
    pub fn text_normal(&self) -> Style {
        Style::default().fg(self.text)
    }

    /// Style for secondary/dimmed text.
    pub fn text_dim(&self) -> Style {
        Style::default().fg(self.text_secondary)
    }

    /// Style for highlighted/selected list items.
    pub fn list_selected(&self) -> Style {
        Style::default()
            .fg(self.accent_text)
            .bg(self.surface_1)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for success indicators.
    pub fn status_ok(&self) -> Style {
        Style::default().fg(self.success)
    }

    /// Style for error indicators.
    pub fn status_error(&self) -> Style {
        Style::default().fg(self.error)
    }

    /// Style for warning indicators.
    pub fn status_warn(&self) -> Style {
        Style::default().fg(self.warning)
    }

    /// Style for gauge/progress-bar fill.
    pub fn gauge_fill(&self) -> Style {
        Style::default().fg(self.accent)
    }

    /// Create a [`Block`] with the theme's border and title style.
    pub fn styled_block<'a>(&self, title: &'a str) -> Block<'a> {
        Block::default()
            .title(title)
            .title_style(self.block_title())
            .borders(Borders::ALL)
            .border_style(self.block_border())
    }

    /// Create a focused/active [`Block`].
    pub fn styled_block_active<'a>(&self, title: &'a str) -> Block<'a> {
        Block::default()
            .title(title)
            .title_style(self.block_title())
            .borders(Borders::ALL)
            .border_style(self.block_border_active())
    }
}

impl Default for TuiTheme {
    fn default() -> Self {
        Self::from_cli_theme(ThemeName::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// All 12 themes must construct without panic.
    #[test]
    fn all_themes_construct() {
        let names = [
            ThemeName::Cyan,
            ThemeName::Sgcc,
            ThemeName::Blue,
            ThemeName::Indigo,
            ThemeName::Violet,
            ThemeName::Emerald,
            ThemeName::Amber,
            ThemeName::Coral,
            ThemeName::Rose,
            ThemeName::Teal,
            ThemeName::Sunset,
            ThemeName::Slate,
        ];
        for name in names {
            let theme = TuiTheme::from_cli_theme(name);
            match theme.accent {
                Color::Rgb(_, _, _) => {}
                _ => panic!("Expected Rgb color for {:?}", name),
            }
        }
    }

    /// Default theme should be Cyan.
    #[test]
    fn default_is_coral() {
        let theme = TuiTheme::default();
        assert!(matches!(theme.name, ThemeName::Coral));
        assert_eq!(theme.accent, Color::Rgb(249, 115, 22));
    }

    /// Sunset should have a secondary accent; others should not.
    #[test]
    fn sunset_has_accent2() {
        let sunset = TuiTheme::from_cli_theme(ThemeName::Sunset);
        assert!(sunset.accent2.is_some());

        let cyan = TuiTheme::from_cli_theme(ThemeName::Cyan);
        assert!(cyan.accent2.is_none());
    }

    /// accent_dim should be roughly half of accent.
    #[test]
    fn accent_dim_is_halved() {
        let theme = TuiTheme::from_cli_theme(ThemeName::Blue);
        assert_eq!(theme.accent, Color::Rgb(59, 130, 246));
        assert_eq!(theme.accent_dim, Color::Rgb(29, 65, 123));
    }

    /// accent_glow should be a lighter shade (midpoint with white).
    #[test]
    fn accent_glow_is_lighter() {
        let theme = TuiTheme::from_cli_theme(ThemeName::Cyan);
        // (6+255)/2=130, (182+255)/2=218, (212+255)/2=233
        assert_eq!(theme.accent_glow, Color::Rgb(130, 218, 233));
    }

    /// Style methods should return valid Style values.
    #[test]
    fn style_methods_return_styles() {
        let theme = TuiTheme::default();

        let active = theme.tab_active();
        assert!(active.fg.is_some());

        let inactive = theme.tab_inactive();
        assert!(inactive.fg.is_some());

        let title = theme.block_title();
        assert!(title.fg.is_some());

        let border = theme.block_border();
        assert!(border.fg.is_some());

        let border_active = theme.block_border_active();
        assert!(border_active.fg.is_some());

        let normal = theme.text_normal();
        assert!(normal.fg.is_some());

        let dim = theme.text_dim();
        assert!(dim.fg.is_some());

        let selected = theme.list_selected();
        assert!(selected.fg.is_some());
        assert!(selected.bg.is_some());

        let ok = theme.status_ok();
        assert!(ok.fg.is_some());

        let err = theme.status_error();
        assert!(err.fg.is_some());

        let warn = theme.status_warn();
        assert!(warn.fg.is_some());

        let gauge = theme.gauge_fill();
        assert!(gauge.fg.is_some());
    }

    /// Semantic colors should be consistent across all themes.
    #[test]
    fn semantic_colors_are_consistent() {
        let cyan = TuiTheme::from_cli_theme(ThemeName::Cyan);
        let rose = TuiTheme::from_cli_theme(ThemeName::Rose);

        assert_eq!(cyan.success, rose.success);
        assert_eq!(cyan.error, rose.error);
        assert_eq!(cyan.warning, rose.warning);
        assert_eq!(cyan.info, rose.info);
        assert_eq!(cyan.muted, rose.muted);
        assert_eq!(cyan.surface, rose.surface);
        assert_eq!(cyan.border, rose.border);
        assert_eq!(cyan.text, rose.text);
        assert_eq!(cyan.text_faint, rose.text_faint);
        assert_eq!(cyan.surface_1, rose.surface_1);
        assert_eq!(cyan.surface_2, rose.surface_2);
        assert_eq!(cyan.surface_3, rose.surface_3);
    }

    /// Surface layers should increase in brightness.
    #[test]
    fn surface_layers_increase_brightness() {
        let theme = TuiTheme::default();
        fn brightness(c: Color) -> u16 {
            match c {
                Color::Rgb(r, g, b) => r as u16 + g as u16 + b as u16,
                _ => 0,
            }
        }
        assert!(brightness(theme.surface) < brightness(theme.surface_1));
        assert!(brightness(theme.surface_1) < brightness(theme.surface_2));
        assert!(brightness(theme.surface_2) < brightness(theme.surface_3));
    }

    /// styled_block and styled_block_active should not panic.
    #[test]
    fn styled_blocks_construct() {
        let theme = TuiTheme::default();
        let _block = theme.styled_block("Test");
        let _active = theme.styled_block_active("Active");
    }
}
