use iced::border::{Border, Radius};
use iced::widget::rule;
use iced::widget::{button, column, container, row, text, text_input, Space};
use iced::{Alignment, Background, Element, Length, Shadow};

use crate::app::message::Message;
use crate::app::state::CommandActionId;

use super::super::desktop::CptDesktop;
use super::styles::{ghost_button_style, text_input_style, with_alpha};
use crate::app::theme::Palette;

impl CptDesktop {
    pub(crate) fn command_palette_view(&self) -> Option<Element<'_, Message>> {
        if !self.command_palette.open {
            return None;
        }

        let filtered = self.command_palette.filtered();
        let palette = self.palette;
        let items = if filtered.is_empty() {
            column![text("No matches").size(14).color(palette.text_muted)]
        } else {
            filtered.into_iter().enumerate().fold(
                column![].spacing(6),
                |column, (index, action)| {
                    let content = column![
                        text(action.label).size(16).color(palette.text_primary),
                        text(action.description).size(12).color(palette.text_muted),
                    ]
                    .spacing(4)
                    .align_x(Alignment::Start);

                    let selected = index == self.command_palette.selected;

                    column.push(
                        button(content)
                            .width(Length::Fill)
                            .on_press(Message::CommandPaletteExecute(action.id))
                            .style(move |_, status| menu_button_style(palette, selected, status)),
                    )
                },
            )
        };

        let header = row![
            text("Command palette").size(20).color(palette.text_primary),
            Space::new().width(Length::Fill),
            button(text("Close").color(palette.secondary_text))
                .on_press(Message::CommandPaletteClosed)
                .style(move |_, status| ghost_button_style(palette, status)),
        ]
        .align_y(Alignment::Center);

        let input = text_input("Type a command", &self.command_palette.query)
            .id(self.command_palette_input_id.clone())
            .on_input(Message::CommandPaletteQueryChanged)
            .on_submit(Message::CommandPaletteExecute(
                self.command_palette
                    .selected_action()
                    .map(|action| action.id)
                    .unwrap_or(CommandActionId::Refresh),
            ))
            .style(move |_, status| text_input_style(palette, status));

        Some(
            container(
                column![header, input, rule::horizontal(1), items]
                    .spacing(12)
                    .padding(16),
            )
            .width(Length::Fill)
            .style(move |_| modal_container_style(palette))
            .into(),
        )
    }
}

fn menu_button_style(palette: Palette, selected: bool, status: button::Status) -> button::Style {
    let mut style = button::Style {
        background: Some(Background::Color(if selected {
            palette.sidebar_active
        } else {
            with_alpha(palette.sidebar_hover, 0.3)
        })),
        border: Border {
            color: with_alpha(palette.border, if selected { 0.9 } else { 0.5 }),
            width: 1.0,
            radius: Radius::from(10.0),
        },
        text_color: if selected {
            palette.sidebar_text
        } else {
            palette.sidebar_text_muted
        },
        shadow: Shadow::default(),
        ..button::Style::default()
    };

    match status {
        button::Status::Hovered | button::Status::Pressed => {
            style.background = Some(Background::Color(palette.sidebar_hover));
            style.text_color = palette.sidebar_text;
        }
        button::Status::Disabled => {
            style.text_color = with_alpha(style.text_color, 0.6);
        }
        button::Status::Active => {}
    }

    style
}

fn modal_container_style(palette: Palette) -> container::Style {
    container::Style {
        background: Some(Background::Color(palette.surface_muted)),
        border: Border {
            color: palette.border,
            width: 1.0,
            radius: Radius::from(16.0),
        },
        shadow: Shadow::default(),
        ..container::Style::default()
    }
}
