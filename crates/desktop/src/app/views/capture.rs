use iced::border::{Border, Radius};
use iced::font::Weight as FontWeight;
use iced::widget::{column, container, row, text, text_input, Space};
use iced::{Alignment, Background, Element, Font, Length, Shadow};

use crate::app::message::Message;
use crate::app::state::{CaptureChip, CaptureChipKind};
use crate::app::theme::Palette;

use super::super::desktop::CptDesktop;
use super::styles::{chip_style, text_input_style, with_alpha};

// Capture tokens are the power-user syntax, so keep them discoverable in the desktop capture flow.
const TOKEN_HINTS: [(&str, &str); 10] = [
    ("@context", "Context label (@home, @phone)"),
    ("+project", "Project name (+Website)"),
    ("#tag", "Tag (#ops)"),
    (
        "due:DATE",
        "Due date (today, tomorrow, fri, 2025-01-20, +3d)",
    ),
    ("defer:DATE", "Start date (tomorrow, +1w)"),
    ("t:30m", "Time estimate (minutes or 2h)"),
    ("e:low|med|high", "Energy level"),
    ("p:0-3", "Priority (0=low … 3=high)"),
    ("wait:Name", "Waiting on person/contact"),
    ("since:DATE", "Waiting since (today, +2d)"),
];

impl CptDesktop {
    pub(crate) fn capture_view(&self) -> Element<'_, Message> {
        if !self.capture.open {
            return Space::new().height(Length::Shrink).into();
        }

        let palette = self.palette;
        let input = text_input("Add a task with inline tokens", &self.capture.text)
            .id(self.capture_input_id.clone())
            .on_input(Message::CaptureTextChanged)
            .on_submit(Message::CaptureSubmit)
            .padding(12)
            .style(move |_, status| text_input_style(palette, status));

        let chips_row: Element<'_, Message> = if let Some(preview) = &self.capture.preview {
            preview
                .chips
                .iter()
                .fold(row![].spacing(8).align_y(Alignment::Center), |row, chip| {
                    row.push(capture_chip(chip, palette))
                })
                .into()
        } else {
            Space::new().height(Length::Shrink).into()
        };

        let error = if let Some(err) = &self.capture.preview_error {
            text(err).size(12).color(self.palette.danger)
        } else {
            text("").size(12).color(self.palette.text_muted)
        };

        column![input, chips_row, error, token_hint_section(palette)]
            .spacing(4)
            .into()
    }
}

fn capture_chip(chip: &CaptureChip, palette: Palette) -> Element<'_, Message> {
    use iced::alignment::{Horizontal, Vertical};
    use iced::widget::{container, text};

    let (icon, color) = match chip.kind {
        CaptureChipKind::Project => ("+", with_alpha(palette.info, 0.85)),
        CaptureChipKind::Context => ("@", palette.info),
        CaptureChipKind::Tag => ("#", palette.primary),
        CaptureChipKind::Due => ("due", palette.danger),
        CaptureChipKind::Defer => ("defer", palette.secondary_hover),
        CaptureChipKind::Energy => ("⚡", palette.success),
        CaptureChipKind::Priority => ("P", palette.warning),
        CaptureChipKind::Waiting => ("wait", with_alpha(palette.warning, 0.85)),
    };

    container(
        text(format!("{} {}", icon, chip.label))
            .size(12)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center),
    )
    .padding([4, 8])
    .style(move |_| chip_style(color))
    .into()
}

fn token_hint_section(palette: Palette) -> Element<'static, Message> {
    use iced::widget::{column, row, text};

    let hints = TOKEN_HINTS
        .iter()
        .fold(column![].spacing(6), |column, (token, description)| {
            column.push(
                row![
                    text(*token).size(12).color(palette.info).font(Font {
                        weight: FontWeight::Bold,
                        ..Font::DEFAULT
                    }),
                    text(*description).size(12).color(palette.text_muted),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            )
        });

    container(hints)
        .width(Length::Fill)
        .padding([12, 16])
        .style(move |_| token_hint_container_style(palette))
        .into()
}

fn token_hint_container_style(palette: Palette) -> container::Style {
    container::Style {
        background: Some(Background::Color(with_alpha(palette.surface_muted, 0.85))),
        border: Border {
            color: with_alpha(palette.border, 0.6),
            width: 1.0,
            radius: Radius::from(12.0),
        },
        shadow: Shadow::default(),
        ..container::Style::default()
    }
}
