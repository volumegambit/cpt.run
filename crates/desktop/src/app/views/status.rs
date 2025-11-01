use iced::widget::{row, text, Space};
use iced::{Alignment, Element, Length};

use crate::app::message::Message;
use crate::app::state::{LoadState, ToastKind};

use super::super::desktop::CptDesktop;

impl CptDesktop {
    pub(crate) fn status_line(&self) -> Element<'_, Message> {
        let store = self.views.get(&self.active);
        let mut left = match store.and_then(|view| view.last_refreshed) {
            Some(ts) => text(format!("Last refreshed {}s ago", ts.elapsed().as_secs()))
                .size(12)
                .color(self.palette.text_secondary),
            None => text("Not yet refreshed")
                .size(12)
                .color(self.palette.text_secondary),
        };

        if let Some(store) = store {
            match &store.state {
                LoadState::Loading => {
                    left = text("Loading…").size(12).color(self.palette.info);
                }
                LoadState::Error(err) => {
                    left = text(format!("Error: {err}"))
                        .size(12)
                        .color(self.palette.danger);
                }
                _ => {}
            }
        }

        let right = if let Some(status) = &self.status {
            let color = match status.kind {
                ToastKind::Info => self.palette.info,
                ToastKind::Error => self.palette.danger,
            };
            text(&status.message).size(12).color(color)
        } else {
            text("").size(12).color(self.palette.text_secondary)
        };

        row![left, Space::new().width(Length::Fill), right]
            .align_y(Alignment::Center)
            .into()
    }
}
