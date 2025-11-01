use iced::widget::{button, row, text, Space};
use iced::{Alignment, Element, Length, Theme};

use crate::app::message::Message;

use super::styles::{ghost_button_style, primary_button_style};

use super::super::desktop::CptDesktop;

impl CptDesktop {
    pub(crate) fn toolbar(&self) -> Element<'_, Message> {
        let palette = self.palette;
        let theme_label = match self.theme {
            Theme::Dark => "Switch to light",
            _ => "Switch to dark",
        };

        let capture_button = button(
            row![text("Add Task").color(palette.primary_text).size(14)]
                .spacing(8)
                .align_y(Alignment::Center),
        )
        .on_press(Message::CaptureToggled)
        .style(move |_, status| primary_button_style(palette, status));

        let palette_label = text("Command Palette")
            .size(14)
            .color(palette.secondary_text);
        let palette_hint = text("Cmd+K").size(12).color(palette.text_muted);

        let command_palette_button = button(
            row![palette_label, palette_hint]
                .spacing(8)
                .align_y(Alignment::Center),
        )
        .on_press(Message::CommandPaletteToggled)
        .style(move |_, status| ghost_button_style(palette, status));

        let mut bar = row![capture_button].spacing(16).align_y(Alignment::Center);

        bar = bar.push(Space::new().width(Length::Fill));

        if self.pending_mutations > 0 {
            bar = bar.push(text("Applying changes…").size(14).color(palette.info));
        }

        let theme_button = button(text(theme_label).size(14).color(palette.secondary_text))
            .on_press(Message::ToggleTheme)
            .style(move |_, status| ghost_button_style(palette, status));

        bar = bar.push(theme_button);

        bar = bar.push(command_palette_button);

        bar.into()
    }
}
