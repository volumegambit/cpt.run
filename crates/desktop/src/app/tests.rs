//! Exercised flows ensure keyboard-driven cpt.run workflows stay reliable in the desktop shell.

#[cfg(test)]
mod tests {
    use iced::keyboard::{Event as KeyboardEvent, Key, Location, Modifiers};
    use tempfile::TempDir;

    use cpt_core::model::ListFilters;
    use cpt_core::{AppConfig, TasksService};

    use super::desktop::CptDesktop;
    use super::message::Message;
    use super::options::{DesktopFlags, DesktopOptions};
    use super::seeding::maybe_seed_sample_data;
    use super::state::ViewTab;

    fn init_app() -> (CptDesktop, TasksService, TempDir) {
        let temp_dir = TempDir::new().expect("temp dir");
        let config = AppConfig::from_data_dir(temp_dir.path().to_path_buf()).unwrap();
        let service = TasksService::new(config.clone()).unwrap();
        let _ = maybe_seed_sample_data(&service);

        let flags = DesktopFlags::from(DesktopOptions {
            data_dir: Some(config.data_dir().to_path_buf()),
            ..Default::default()
        });

        let (mut app, _) = CptDesktop::new(flags);
        let snapshot = service
            .list(&ListFilters::for_view(ViewTab::Inbox.list_view()))
            .unwrap();
        let _ = app.update(Message::ViewLoaded(ViewTab::Inbox, Ok(snapshot)));
        (app, service, temp_dir)
    }

    #[test]
    fn ctrl_k_opens_command_palette() {
        let (mut app, _service, _guard) = init_app();

        let event = KeyboardEvent::KeyPressed {
            key: Key::Character("k".into()),
            location: Location::Standard,
            modifiers: Modifiers::COMMAND,
            text: Some("k".into()),
        };

        let _ = app.update(Message::Keyboard(event));
        assert!(app.command_palette.open);
        assert!(!app.capture.open);
    }

    #[test]
    fn selecting_row_tracks_selection() {
        let (mut app, _service, _guard) = init_app();
        let first_id = app
            .current_tasks()
            .first()
            .expect("sample tasks available")
            .id
            .clone();

        let _ = app.update(Message::RowSelected(first_id.clone()));
        assert_eq!(app.selected_task.as_deref(), Some(first_id.as_str()))
    }

    #[test]
    fn mark_done_shortcut_queues_mutation() {
        let (mut app, _service, _guard) = init_app();
        let first_id = app
            .current_tasks()
            .first()
            .expect("sample tasks available")
            .id
            .clone();
        let _ = app.update(Message::RowSelected(first_id.clone()));

        let event = KeyboardEvent::KeyPressed {
            key: Key::Character("d".into()),
            location: Location::Standard,
            modifiers: Modifiers::default(),
            text: Some("d".into()),
        };

        let _ = app.update(Message::Keyboard(event));
        assert!(app.pending_mutations >= 1);
    }
}
