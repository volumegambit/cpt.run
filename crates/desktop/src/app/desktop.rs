//! Iced `Application` implementation powering the cpt.run desktop shell lifecycle.

use std::collections::HashMap;
use std::env;
use std::io::Cursor;
use std::time::{Duration, Instant};

use cpt_core::{AppConfig, TasksService};
use iced::event::{self, Event};
use iced::time;
use iced::widget::Id;
use iced::Subscription;
use iced::{window, Size, Theme};

use crate::app::commands::load_view_command;
use crate::app::helpers::detect_theme;
use crate::app::message::{Effect, Message};
use crate::app::options::{DesktopFlags, DesktopOptions};
use crate::app::seeding::maybe_seed_sample_data;
use crate::app::state::{
    CaptureState, CommandPaletteState, InlineEditState, LoadState, StatusToast, ViewStore, ViewTab,
};
use crate::app::theme::Palette;
use crate::app::views;
use crate::telemetry::{self, Event as TelemetryEvent};

pub fn run(options: DesktopOptions) -> iced::Result {
    let _ = tracing_subscriber::fmt::try_init();

    let boot_flags = DesktopFlags::from(options);
    let window_settings = window::Settings {
        size: Size::new(1140.0, 780.0),
        min_size: Some(Size::new(960.0, 600.0)),
        maximized: true,
        icon: load_window_icon(),
        ..window::Settings::default()
    };

    iced::application(
        move || CptDesktop::bootstrap(boot_flags.clone()),
        CptDesktop::react,
        views::compose_root,
    )
    .window(window_settings)
    .title(app_title)
    .theme(app_theme)
    .subscription(app_subscription)
    .run()
}

fn app_title(_state: &CptDesktop) -> String {
    format!("cpt.run Desktop v{}", env!("CARGO_PKG_VERSION"))
}

fn app_theme(state: &CptDesktop) -> Option<Theme> {
    Some(state.theme.clone())
}

fn app_subscription(state: &CptDesktop) -> Subscription<Message> {
    state.subscription()
}

fn load_window_icon() -> Option<window::Icon> {
    // Embed the desktop brand icon so the native chrome reflects the app identity.
    const ICON_BYTES: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../icons/icon_256x256.png"
    ));

    let decoder = png::Decoder::new(Cursor::new(ICON_BYTES));
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut buf).ok()?;
    let bytes = &buf[..frame.buffer_size()];

    window::icon::from_rgba(bytes.to_vec(), frame.width, frame.height).ok()
}

pub(crate) struct CptDesktop {
    pub(crate) service: Option<TasksService>,
    pub(crate) views: HashMap<ViewTab, ViewStore>,
    pub(crate) active: ViewTab,
    pub(crate) theme: Theme,
    pub(crate) palette: Palette,
    pub(crate) telemetry: telemetry::Handle,
    pub(crate) refresh_interval: Duration,
    pub(crate) status: Option<StatusToast>,
    pub(crate) capture: CaptureState,
    pub(crate) capture_input_id: Id,
    pub(crate) command_palette: CommandPaletteState,
    pub(crate) command_palette_input_id: Id,
    pub(crate) selected_task: Option<String>,
    pub(crate) pending_mutations: usize,
    pub(crate) inline_edit: Option<InlineEditState>,
    pub(crate) last_title_click: Option<(String, Instant)>,
}

impl CptDesktop {
    fn bootstrap(flags: DesktopFlags) -> (Self, Effect) {
        let theme = detect_theme();
        let palette = Palette::for_theme(&theme);
        let telemetry = telemetry::Handle::new();
        let mut views = HashMap::new();
        for tab in ViewTab::ALL {
            views.insert(*tab, ViewStore::new());
        }

        let mut service_opt = None;
        let mut effect = Effect::none();

        match AppConfig::discover(flags.data_dir.clone()) {
            Ok(config) => match TasksService::new(config.clone()) {
                Ok(service) => {
                    telemetry.record(TelemetryEvent::AppStarted);
                    if should_seed_sample_data(&flags, &config) {
                        match maybe_seed_sample_data(&service) {
                            Ok(true) => tracing::debug!("seeded desktop sample data"),
                            Ok(false) => {}
                            Err(err) => {
                                tracing::warn!(error = %err, "failed to seed desktop sample data")
                            }
                        }
                    }
                    views
                        .entry(ViewTab::Inbox)
                        .and_modify(|store| store.state = LoadState::Loading);
                    effect = load_view_command(service.clone(), ViewTab::Inbox);
                    service_opt = Some(service);
                }
                Err(err) => {
                    if let Some(store) = views.get_mut(&ViewTab::Inbox) {
                        store.state = LoadState::Error(err.to_string());
                    }
                }
            },
            Err(err) => {
                if let Some(store) = views.get_mut(&ViewTab::Inbox) {
                    store.state = LoadState::Error(err.to_string());
                }
            }
        }

        (
            Self {
                service: service_opt,
                views,
                active: ViewTab::Inbox,
                theme,
                palette,
                telemetry,
                refresh_interval: flags.refresh_interval,
                status: None,
                capture: CaptureState::new(),
                capture_input_id: Id::new("capture_input"),
                command_palette: CommandPaletteState::new(),
                command_palette_input_id: Id::new("command_palette_input"),
                selected_task: None,
                pending_mutations: 0,
                inline_edit: None,
                last_title_click: None,
            },
            effect,
        )
    }
}

fn should_seed_sample_data(flags: &DesktopFlags, config: &AppConfig) -> bool {
    if !cfg!(debug_assertions) {
        return false;
    }

    if flags.data_dir.is_some() || env::var("CPT_DATA_DIR").is_ok() {
        return false;
    }

    match config.data_dir().file_name().and_then(|name| name.to_str()) {
        Some("dev-cpt") => true,
        _ => false,
    }
}

impl CptDesktop {
    pub(crate) fn subscription(&self) -> Subscription<Message> {
        let refresh = if self.service.is_some() {
            time::every(self.refresh_interval).map(|_| Message::RefreshTick)
        } else {
            Subscription::none()
        };

        let keyboard = event::listen_with(|event, _, _| match event {
            Event::Keyboard(key_event) => Some(Message::Keyboard(key_event)),
            _ => None,
        });

        Subscription::batch(vec![refresh, keyboard])
    }

    pub(super) fn ensure_view_entry(&mut self, tab: ViewTab) {
        self.views.entry(tab).or_insert_with(ViewStore::new);
    }

    pub(super) fn prune_toast(&mut self) {
        if let Some(toast) = &self.status {
            if toast.created_at.elapsed() > Duration::from_secs(6) {
                self.status = None;
            }
        }
    }
}
