use iced::border::{Border, Radius};
use iced::widget::{button, column, text};
use iced::{Alignment, Background, Element, Length, Shadow};

use crate::app::message::Message;
use crate::app::state::ViewTab;

use super::super::desktop::CptDesktop;
use super::styles::with_alpha;

impl CptDesktop {
    pub(crate) fn tabs(&self) -> Element<'_, Message> {
        let palette = self.palette;
        let mut menu = column![].spacing(12).align_x(Alignment::Start);

        for tab in ViewTab::ALL {
            let active = *tab == self.active;
            let title_color = if active {
                palette.sidebar_text
            } else {
                palette.sidebar_text_muted
            };
            let subtitle_color = if active {
                with_alpha(palette.sidebar_text_muted, 0.85)
            } else {
                with_alpha(palette.sidebar_text_muted, 0.65)
            };

            let label = column![
                text(tab.title()).size(16).color(title_color),
                text(tab.subtitle()).size(12).color(subtitle_color),
            ]
            .spacing(2)
            .align_x(Alignment::Start);

            let button = button(label)
                .padding([5, 8])
                .width(Length::Fill)
                .on_press(Message::ViewRequested(*tab))
                .style(move |_, status| sidebar_button_style(palette, active, status));

            menu = menu.push(button);
        }

        menu.width(Length::Fill).into()
    }
}

use crate::app::theme::Palette;

fn sidebar_button_style(palette: Palette, active: bool, status: button::Status) -> button::Style {
    let mut style = button::Style {
        background: None,
        border: Border::default(),
        text_color: if active {
            palette.sidebar_text
        } else {
            palette.sidebar_text_muted
        },
        shadow: Shadow::default(),
        ..button::Style::default()
    };

    if active {
        style.background = Some(Background::Color(palette.sidebar_active));
        style.border = Border {
            radius: Radius::from(6),
            color: palette.sidebar_border,
            width: 1.0,
        };
    }

    match status {
        button::Status::Hovered | button::Status::Pressed => {
            if !active {
                style.background = Some(Background::Color(palette.sidebar_hover));
                style.text_color = palette.sidebar_text;
            }
        }
        button::Status::Disabled => {
            style.text_color = with_alpha(style.text_color, 0.6);
        }
        button::Status::Active => {}
    }

    style
}
