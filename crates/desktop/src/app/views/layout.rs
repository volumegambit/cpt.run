use iced::alignment::Horizontal;
use iced::border::{Border, Radius};
use iced::widget::rule;
use iced::widget::{column, container, row};
use iced::{Alignment, Background, Element, Length, Shadow};

use crate::app::message::Message;
use crate::app::theme::Palette;

use super::super::desktop::CptDesktop;

pub(crate) fn compose(app: &CptDesktop) -> Element<'_, Message> {
    let toolbar = app.toolbar();
    let status_line = app.status_line();
    let capture = app.capture_view();
    let task_list = app.task_list();

    let mut main_column = column![capture, task_list]
        .spacing(16)
        .align_x(Alignment::Start);

    if app.command_palette.open {
        if let Some(palette) = app.command_palette_view() {
            main_column = main_column.push(palette);
        }
    }

    let sidebar = container(app.tabs())
        .width(Length::Fixed(280.0))
        .height(Length::Fill)
        .padding([5, 5])
        .align_x(Horizontal::Left)
        .style(move |_| sidebar_container_style(app.palette));

    let toolbar = container(toolbar)
        .width(Length::Fill)
        .padding([5, 5])
        .style(move |_| toolbar_container_style(app.palette));

    let toolbar_divider = rule::horizontal(1).style(move |_| toolbar_divider_style(app.palette));

    let content = container(main_column)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding([20, 24])
        .style(move |_| surface_container_style(app.palette));

    let main_area = column![toolbar, toolbar_divider, content]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill);

    let body = row![sidebar, main_area]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill);

    let status = container(status_line)
        .width(Length::Fill)
        .padding([8, 24])
        .style(move |_| status_container_style(app.palette));

    container(
        column![body, status]
            .spacing(0)
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(Horizontal::Left)
    .style(move |_| app_background_style(app.palette))
    .into()
}

fn sidebar_container_style(palette: Palette) -> container::Style {
    container::Style {
        background: Some(Background::Color(palette.sidebar_background)),
        border: Border {
            color: palette.sidebar_border,
            width: 1.0,
            radius: Radius::from(0.0),
        },
        shadow: Shadow::default(),
        ..container::Style::default()
    }
}

fn surface_container_style(palette: Palette) -> container::Style {
    container::Style {
        background: Some(Background::Color(palette.surface)),
        border: Border {
            color: palette.border,
            width: 0.0,
            radius: Radius::from(0.0),
        },
        shadow: Shadow::default(),
        ..container::Style::default()
    }
}

fn toolbar_container_style(palette: Palette) -> container::Style {
    container::Style {
        background: Some(Background::Color(palette.surface_muted)),
        border: Border {
            color: palette.border,
            width: 0.0,
            radius: Radius::from(0.0),
        },
        shadow: Shadow::default(),
        ..container::Style::default()
    }
}

fn toolbar_divider_style(palette: Palette) -> rule::Style {
    rule::Style {
        color: palette.border,
        radius: Radius::from(0.0),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }
}

fn status_container_style(palette: Palette) -> container::Style {
    container::Style {
        background: Some(Background::Color(palette.surface_muted)),
        border: Border {
            color: palette.border,
            width: 0.0,
            radius: Radius::from(0.0),
        },
        shadow: Shadow::default(),
        ..container::Style::default()
    }
}

fn app_background_style(palette: Palette) -> container::Style {
    container::Style {
        background: Some(Background::Color(palette.background)),
        border: Border::default(),
        shadow: Shadow::default(),
        ..container::Style::default()
    }
}
