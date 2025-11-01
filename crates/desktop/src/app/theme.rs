//! Palette definitions so the desktop shell matches the cpt.run brand language.

use iced::Color;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Palette {
    pub(crate) background: Color,
    pub(crate) surface: Color,
    pub(crate) surface_muted: Color,
    pub(crate) sidebar_background: Color,
    pub(crate) sidebar_border: Color,
    pub(crate) sidebar_hover: Color,
    pub(crate) sidebar_active: Color,
    pub(crate) sidebar_text: Color,
    pub(crate) sidebar_text_muted: Color,
    pub(crate) primary: Color,
    pub(crate) primary_hover: Color,
    pub(crate) primary_text: Color,
    pub(crate) secondary_hover: Color,
    pub(crate) secondary_text: Color,
    pub(crate) ghost_hover: Color,
    pub(crate) success: Color,
    pub(crate) warning: Color,
    pub(crate) danger: Color,
    pub(crate) info: Color,
    pub(crate) text_primary: Color,
    pub(crate) text_secondary: Color,
    pub(crate) text_muted: Color,
    pub(crate) border: Color,
}

impl Palette {
    pub(crate) fn for_theme(theme: &iced::Theme) -> Self {
        match theme {
            iced::Theme::Dark => Self {
                // Desktop aligns with the TUI aesthetic: near-black panels with lime accents.
                background: Color::from_rgb(0.02, 0.02, 0.03),
                surface: Color::from_rgb(0.05, 0.06, 0.07),
                surface_muted: Color::from_rgb(0.07, 0.09, 0.10),
                sidebar_background: Color::from_rgb(0.04, 0.04, 0.05),
                sidebar_border: Color::from_rgba(0.35, 0.90, 0.35, 0.25),
                sidebar_hover: Color::from_rgba(0.20, 0.55, 0.20, 0.45),
                sidebar_active: Color::from_rgba(0.12, 0.45, 0.12, 0.75),
                sidebar_text: Color::from_rgb(0.95, 0.99, 0.95),
                sidebar_text_muted: Color::from_rgba(0.72, 0.85, 0.72, 0.78),
                primary: Color::from_rgb(0.20, 0.80, 0.20),
                primary_hover: Color::from_rgb(0.28, 0.92, 0.28),
                primary_text: Color::from_rgb(0.98, 0.98, 0.98),
                secondary_hover: Color::from_rgba(0.24, 0.75, 0.24, 0.35),
                secondary_text: Color::from_rgb(0.62, 0.86, 0.62),
                ghost_hover: Color::from_rgba(0.14, 0.55, 0.14, 0.22),
                success: Color::from_rgb(0.20, 0.80, 0.20),
                warning: Color::from_rgb(0.88, 0.74, 0.24),
                danger: Color::from_rgb(0.88, 0.32, 0.32),
                info: Color::from_rgb(0.32, 0.88, 0.70),
                text_primary: Color::from_rgb(0.85, 0.94, 0.85),
                text_secondary: Color::from_rgb(0.55, 0.68, 0.55),
                text_muted: Color::from_rgb(0.36, 0.44, 0.36),
                border: Color::from_rgba(0.24, 0.70, 0.24, 0.35),
            },
            _ => Self {
                background: Color::from_rgb(0.03, 0.03, 0.04),
                surface: Color::from_rgb(0.06, 0.06, 0.07),
                surface_muted: Color::from_rgb(0.08, 0.09, 0.10),
                sidebar_background: Color::from_rgb(0.04, 0.04, 0.05),
                sidebar_border: Color::from_rgba(0.38, 0.92, 0.38, 0.28),
                sidebar_hover: Color::from_rgba(0.22, 0.60, 0.22, 0.45),
                sidebar_active: Color::from_rgba(0.14, 0.52, 0.14, 0.78),
                sidebar_text: Color::from_rgb(0.95, 0.99, 0.95),
                sidebar_text_muted: Color::from_rgba(0.70, 0.84, 0.70, 0.78),
                primary: Color::from_rgb(0.24, 0.88, 0.24),
                primary_hover: Color::from_rgb(0.30, 0.98, 0.30),
                primary_text: Color::from_rgb(0.02, 0.02, 0.03),
                secondary_hover: Color::from_rgba(0.30, 0.88, 0.30, 0.35),
                secondary_text: Color::from_rgb(0.62, 0.86, 0.62),
                ghost_hover: Color::from_rgba(0.18, 0.62, 0.18, 0.22),
                success: Color::from_rgb(0.24, 0.88, 0.24),
                warning: Color::from_rgb(0.94, 0.78, 0.26),
                danger: Color::from_rgb(0.90, 0.30, 0.32),
                info: Color::from_rgb(0.34, 0.90, 0.74),
                text_primary: Color::from_rgb(0.85, 0.94, 0.85),
                text_secondary: Color::from_rgb(0.55, 0.68, 0.55),
                text_muted: Color::from_rgb(0.36, 0.44, 0.36),
                border: Color::from_rgba(0.26, 0.72, 0.26, 0.32),
            },
        }
    }
}
