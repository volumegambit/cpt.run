use iced::border::{Border, Radius};
use iced::widget::{button, container, text_input};
use iced::{Background, Color, Shadow, Vector};

use crate::app::theme::Palette;

pub(super) fn with_alpha(color: Color, alpha: f32) -> Color {
    Color { a: alpha, ..color }
}

pub(super) fn darken(color: Color, factor: f32) -> Color {
    let clamp = |value: f32| value.clamp(0.0, 1.0);
    Color {
        r: clamp(color.r * factor),
        g: clamp(color.g * factor),
        b: clamp(color.b * factor),
        ..color
    }
}

pub(super) fn primary_button_style(palette: Palette, status: button::Status) -> button::Style {
    let base = darken(palette.primary, 0.85);
    let mut style = button::Style {
        background: Some(Background::Color(base)),
        border: Border {
            color: base,
            width: 0.0,
            radius: Radius::from(2.0),
        },
        text_color: palette.primary_text,
        shadow: Shadow {
            offset: Vector::new(0.0, 1.0),
            ..Shadow::default()
        },
        ..button::Style::default()
    };

    match status {
        button::Status::Hovered => {
            style.background = Some(Background::Color(palette.primary));
            style.border.color = palette.primary;
        }
        button::Status::Pressed => {
            let pressed = darken(palette.primary, 0.75);
            style.background = Some(Background::Color(pressed));
            style.border.color = pressed;
            style.shadow.offset = Vector::new(0.0, 0.0);
        }
        button::Status::Disabled => {
            let disabled_base = with_alpha(base, 0.6);
            style.background = Some(Background::Color(disabled_base));
            style.border.color = disabled_base;
            style.text_color = with_alpha(palette.primary_text, 0.6);
            style.shadow.offset = Vector::new(0.0, 0.0);
        }
        button::Status::Active => {}
    }

    style
}

pub(super) fn ghost_button_style(palette: Palette, status: button::Status) -> button::Style {
    let mut style = button::Style {
        background: None,
        border: Border::default(),
        text_color: palette.secondary_text,
        shadow: Shadow::default(),
        ..button::Style::default()
    };

    match status {
        button::Status::Hovered | button::Status::Pressed => {
            style.background = Some(Background::Color(palette.ghost_hover));
            style.text_color = palette.text_primary;
        }
        button::Status::Disabled => {
            style.text_color = with_alpha(palette.secondary_text, 0.6);
        }
        button::Status::Active => {}
    }

    style
}

pub(super) fn text_input_style(palette: Palette, status: text_input::Status) -> text_input::Style {
    let mut style = text_input::Style {
        background: Background::Color(palette.surface_muted),
        border: Border {
            color: palette.border,
            width: 1.0,
            radius: Radius::from(6.0),
        },
        icon: palette.text_secondary,
        placeholder: palette.text_muted,
        value: palette.text_primary,
        selection: with_alpha(palette.primary, 0.35),
    };

    match status {
        text_input::Status::Focused { is_hovered } => {
            style.border.color = if is_hovered {
                palette.primary_hover
            } else {
                palette.primary
            };
        }
        text_input::Status::Hovered => {
            style.border.color = palette.primary_hover;
        }
        text_input::Status::Disabled => {
            style.background = Background::Color(with_alpha(palette.surface_muted, 0.6));
            style.border.color = with_alpha(palette.border, 0.3);
            style.value = with_alpha(palette.text_primary, 0.6);
            style.placeholder = with_alpha(palette.text_muted, 0.5);
            style.icon = with_alpha(palette.text_secondary, 0.5);
        }
        text_input::Status::Active => {}
    }

    style
}

pub(super) fn chip_style(color: Color) -> container::Style {
    let mut fill = color;
    fill.a = 0.18;

    container::Style {
        background: Some(Background::Color(fill)),
        border: Border {
            color,
            width: 0.0,
            radius: Radius::from(8.0),
        },
        text_color: Some(color),
        shadow: Shadow::default(),
        ..container::Style::default()
    }
}
