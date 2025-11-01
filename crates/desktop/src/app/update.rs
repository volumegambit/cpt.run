//! Core update loop translating user interactions into state changes.

use std::collections::BTreeSet;
use std::time::{Duration as StdDuration, Instant};

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use cpt_core::model::{AddOutcome, Task, TaskStatus};
use iced::keyboard::{key::Named, Event as KeyboardEvent, Key};
use iced::widget::operation::{focus, move_cursor_to_end};
use iced::widget::Id;
use iced::Theme;

use crate::app::commands::{capture_command, load_view_command, mutation_command};
use crate::app::helpers::capitalize;
use crate::app::message::{Effect, Message};
use crate::app::state::{
    CommandActionId, InlineEditState, InlineEditableField, LoadState, MutationKind, StatusToast,
    ToastKind, ViewTab,
};
use crate::app::theme::Palette;
use crate::telemetry::Event as TelemetryEvent;

use super::desktop::CptDesktop;

const TITLE_DOUBLE_CLICK_WINDOW: StdDuration = StdDuration::from_millis(350);
const NONE_OPTION_LABEL: &str = "(none)";
const PRIORITY_CHOICES: &[(u8, &str)] = &[(0, "None"), (1, "Low"), (2, "Medium"), (3, "High")];

impl CptDesktop {
    pub(super) fn react(&mut self, message: Message) -> Effect {
        self.prune_toast();
        match message {
            Message::ViewRequested(tab) => self.switch_view(tab),
            Message::ViewLoaded(tab, result) => self.handle_view_loaded(tab, result),
            Message::RefreshTick => self.on_refresh_tick(),
            Message::ToggleTheme => self.toggle_theme(),
            Message::CaptureToggled => self.toggle_capture(),
            Message::CaptureTextChanged(value) => {
                self.capture.on_text_changed(value);
                Effect::none()
            }
            Message::CaptureSubmit => self.submit_capture(),
            Message::CaptureCompleted(result) => self.finish_capture(result),
            Message::CommandPaletteToggled => self.toggle_command_palette(),
            Message::CommandPaletteClosed => {
                self.command_palette.close();
                Effect::none()
            }
            Message::CommandPaletteQueryChanged(value) => {
                self.command_palette.query = value;
                self.command_palette.clamp_selection();
                Effect::none()
            }
            Message::CommandPaletteExecute(action) => {
                self.command_palette.close();
                self.handle_action(action)
            }
            Message::MutationFinished(kind, result) => self.finish_mutation(kind, result),
            Message::RowSelected(id) => {
                if self
                    .inline_edit
                    .as_ref()
                    .map(|edit| edit.task_id != id)
                    .unwrap_or(false)
                {
                    self.inline_edit = None;
                }
                self.selected_task = Some(id);
                Effect::none()
            }
            Message::TaskTitlePressed(id) => self.handle_title_press(id),
            Message::TaskProjectPressed(id) => {
                self.start_field_edit(id, InlineEditableField::Project)
            }
            Message::TaskContextsPressed(id) => {
                self.start_field_edit(id, InlineEditableField::Contexts)
            }
            Message::TaskTagsPressed(id) => self.start_field_edit(id, InlineEditableField::Tags),
            Message::TaskPriorityPressed(id) => {
                self.start_field_edit(id, InlineEditableField::Priority)
            }
            Message::InlineEditChanged(value) => {
                self.update_inline_edit(value);
                Effect::none()
            }
            Message::InlineEditSubmitted => self.submit_inline_edit(),
            Message::InlineEditOptionSelected(option) => self.handle_inline_option(option),
            Message::Keyboard(event) => self.handle_keyboard(event),
        }
    }

    pub(super) fn handle_title_press(&mut self, id: String) -> Effect {
        let now = Instant::now();
        let is_double_click = self
            .last_title_click
            .as_ref()
            .filter(|(prev_id, prev_time)| {
                prev_id == &id && now.duration_since(*prev_time) <= TITLE_DOUBLE_CLICK_WINDOW
            })
            .is_some();

        self.last_title_click = Some((id.clone(), now));
        self.selected_task = Some(id.clone());

        if is_double_click {
            self.last_title_click = None;
            return self.start_title_edit(id);
        }

        Effect::none()
    }

    pub(super) fn handle_view_loaded(
        &mut self,
        tab: ViewTab,
        result: Result<cpt_core::ViewSnapshot, String>,
    ) -> Effect {
        if let Some(store) = self.views.get_mut(&tab) {
            match result {
                Ok(snapshot) => {
                    let task_count = snapshot.tasks.len();
                    store.last_refreshed = Some(Instant::now());
                    store.state = LoadState::Idle;
                    store.version = store.version.wrapping_add(1);
                    store.snapshot = Some(snapshot);
                    self.telemetry.record(TelemetryEvent::RefreshCompleted {
                        view: tab.title().into(),
                        count: task_count,
                    });
                    if tab == self.active {
                        self.sync_selection_with_view();
                    }
                }
                Err(err) => {
                    store.state = LoadState::Error(err.clone());
                    self.telemetry.record(TelemetryEvent::RefreshFailed {
                        view: tab.title().into(),
                        error: err.clone(),
                    });
                    self.status = Some(StatusToast {
                        message: err,
                        kind: ToastKind::Error,
                        created_at: Instant::now(),
                    });
                }
            }
        }
        Effect::none()
    }

    pub(super) fn on_refresh_tick(&mut self) -> Effect {
        if self.pending_mutations == 0 {
            self.refresh_active_view()
        } else {
            Effect::none()
        }
    }

    pub(super) fn toggle_theme(&mut self) -> Effect {
        self.theme = match self.theme {
            Theme::Dark => Theme::Light,
            _ => Theme::Dark,
        };
        self.palette = Palette::for_theme(&self.theme);
        Effect::none()
    }

    pub(super) fn toggle_capture(&mut self) -> Effect {
        self.capture.toggle();
        if self.capture.open {
            self.command_palette.close();
            return self.focus_capture_input();
        }
        Effect::none()
    }

    pub(super) fn toggle_command_palette(&mut self) -> Effect {
        self.command_palette.toggle();
        if self.command_palette.open {
            self.capture.open = false;
            return Effect::batch(vec![
                focus(self.command_palette_input_id.clone()),
                move_cursor_to_end(self.command_palette_input_id.clone()),
            ]);
        }
        if self.capture.open {
            self.focus_capture_input()
        } else {
            Effect::none()
        }
    }

    pub(super) fn start_title_edit(&mut self, id: String) -> Effect {
        self.start_field_edit(id, InlineEditableField::Title)
    }

    pub(super) fn start_field_edit(&mut self, id: String, field: InlineEditableField) -> Effect {
        if self
            .inline_edit
            .as_ref()
            .map(|edit| edit.task_id == id && edit.field == field)
            .unwrap_or(false)
        {
            return Effect::none();
        }

        let Some(edit) = self.build_inline_edit(&id, field) else {
            return Effect::none();
        };

        self.last_title_click = None;

        let input_id = edit.input_id.clone();
        self.inline_edit = Some(edit);
        self.selected_task = Some(id);
        Effect::batch(vec![focus(input_id.clone()), move_cursor_to_end(input_id)])
    }

    fn build_inline_edit(&self, id: &str, field: InlineEditableField) -> Option<InlineEditState> {
        let tasks = self.current_tasks();
        let task = tasks.into_iter().find(|task| task.id == id)?;
        let input_id = Id::unique();

        match field {
            InlineEditableField::Title => {
                let value = task.title.clone();
                Some(InlineEditState {
                    task_id: id.to_string(),
                    field,
                    value: value.clone(),
                    original_value: value,
                    input_id,
                    options: Vec::new(),
                    original_tokens: Vec::new(),
                })
            }
            InlineEditableField::Project => {
                let mut options = self.collect_projects();
                if !options.iter().any(|option| option == NONE_OPTION_LABEL) {
                    options.insert(0, NONE_OPTION_LABEL.into());
                }
                let value = task
                    .project
                    .as_ref()
                    .map(|project| project.trim())
                    .filter(|project| !project.is_empty())
                    .map(|project| project.to_string())
                    .unwrap_or_default();
                Some(InlineEditState {
                    task_id: id.to_string(),
                    field,
                    value: value.clone(),
                    original_value: value,
                    input_id,
                    options,
                    original_tokens: Vec::new(),
                })
            }
            InlineEditableField::Contexts => {
                let original_tokens: Vec<String> = task
                    .contexts
                    .iter()
                    .map(|ctx| ctx.trim().to_string())
                    .filter(|ctx| !ctx.is_empty())
                    .collect();
                let display = original_tokens.join(", ");
                Some(InlineEditState {
                    task_id: id.to_string(),
                    field,
                    value: display.clone(),
                    original_value: display,
                    input_id,
                    options: self.collect_contexts(),
                    original_tokens,
                })
            }
            InlineEditableField::Tags => {
                let original_tokens: Vec<String> = task
                    .tags
                    .iter()
                    .map(|tag| tag.trim().to_string())
                    .filter(|tag| !tag.is_empty())
                    .collect();
                let display = original_tokens.join(", ");
                Some(InlineEditState {
                    task_id: id.to_string(),
                    field,
                    value: display.clone(),
                    original_value: display,
                    input_id,
                    options: self.collect_tags(),
                    original_tokens,
                })
            }
            InlineEditableField::Priority => {
                let options = priority_options();
                let current_priority = task.priority;
                let display = priority_label(current_priority);
                Some(InlineEditState {
                    task_id: id.to_string(),
                    field,
                    value: display.clone(),
                    original_value: display,
                    input_id,
                    options,
                    original_tokens: Vec::new(),
                })
            }
        }
    }

    pub(super) fn update_inline_edit(&mut self, value: String) {
        if let Some(edit) = self.inline_edit.as_mut() {
            edit.value = value;
        }
    }

    pub(super) fn handle_inline_option(&mut self, option: String) -> Effect {
        let Some(current) = self.inline_edit.clone() else {
            return Effect::none();
        };

        match current.field {
            InlineEditableField::Project => {
                if let Some(edit) = self.inline_edit.as_mut() {
                    edit.value = if option == NONE_OPTION_LABEL {
                        String::new()
                    } else {
                        option.clone()
                    };
                }
                self.submit_inline_edit()
            }
            InlineEditableField::Contexts | InlineEditableField::Tags => {
                if let Some(edit) = self.inline_edit.as_mut() {
                    let mut tokens = parse_token_list(&edit.value);
                    let candidate = option.trim();
                    if !candidate.is_empty()
                        && !tokens
                            .iter()
                            .any(|existing| existing.eq_ignore_ascii_case(candidate))
                    {
                        tokens.push(candidate.to_string());
                        edit.value = tokens.join(", ");
                    }
                }
                Effect::none()
            }
            InlineEditableField::Priority => {
                if let Some(edit) = self.inline_edit.as_mut() {
                    edit.value = option.clone();
                }
                self.submit_inline_edit()
            }
            InlineEditableField::Title => Effect::none(),
        }
    }

    pub(super) fn cancel_inline_edit(&mut self) {
        self.inline_edit = None;
        self.last_title_click = None;
    }
    pub(super) fn submit_inline_edit(&mut self) -> Effect {
        let Some(edit) = self.inline_edit.clone() else {
            return Effect::none();
        };

        match edit.field {
            InlineEditableField::Title => {
                let trimmed = edit.value.trim();
                if trimmed.is_empty() {
                    self.status = Some(StatusToast {
                        message: "Task title cannot be empty".into(),
                        kind: ToastKind::Error,
                        created_at: Instant::now(),
                    });
                    return Effect::none();
                }

                if trimmed == edit.original_value {
                    self.inline_edit = None;
                    return Effect::none();
                }

                let task_id = edit.task_id.clone();
                let title = trimmed.to_string();
                self.inline_edit = None;
                if let Some(service) = self.service.clone() {
                    let kind = MutationKind::Rename {
                        id: task_id.clone(),
                        title: title.clone(),
                    };
                    self.apply_optimistic_update(&[task_id.clone()], &kind);
                    self.pending_mutations += 1;
                    return Effect::perform(
                        mutation_command(service, kind.clone()),
                        move |result| Message::MutationFinished(kind.clone(), result),
                    );
                } else {
                    self.apply_optimistic_title(&task_id, &title);
                    return Effect::none();
                }
            }
            InlineEditableField::Project => {
                let trimmed = edit.value.trim();
                let project = if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                };
                let original_trimmed = edit.original_value.trim();
                let original_project = if original_trimmed.is_empty() {
                    None
                } else {
                    Some(original_trimmed.to_string())
                };
                if project == original_project {
                    self.inline_edit = None;
                    return Effect::none();
                }

                let task_id = edit.task_id.clone();
                self.inline_edit = None;
                if let Some(service) = self.service.clone() {
                    let kind = MutationKind::ChangeProject {
                        id: task_id.clone(),
                        project: project.clone(),
                    };
                    self.apply_optimistic_update(&[task_id.clone()], &kind);
                    self.pending_mutations += 1;
                    return Effect::perform(
                        mutation_command(service, kind.clone()),
                        move |result| Message::MutationFinished(kind.clone(), result),
                    );
                } else {
                    self.apply_optimistic_project(&task_id, project);
                    return Effect::none();
                }
            }
            InlineEditableField::Contexts => {
                let tokens = parse_token_list(&edit.value);
                if tokens == edit.original_tokens {
                    self.inline_edit = None;
                    return Effect::none();
                }
                let task_id = edit.task_id.clone();
                self.inline_edit = None;
                if let Some(service) = self.service.clone() {
                    let kind = MutationKind::ChangeContexts {
                        id: task_id.clone(),
                        contexts: tokens.clone(),
                    };
                    self.apply_optimistic_update(&[task_id.clone()], &kind);
                    self.pending_mutations += 1;
                    return Effect::perform(
                        mutation_command(service, kind.clone()),
                        move |result| Message::MutationFinished(kind.clone(), result),
                    );
                } else {
                    self.apply_optimistic_contexts(&task_id, &tokens);
                    return Effect::none();
                }
            }
            InlineEditableField::Tags => {
                let tokens = parse_token_list(&edit.value);
                if tokens == edit.original_tokens {
                    self.inline_edit = None;
                    return Effect::none();
                }
                let task_id = edit.task_id.clone();
                self.inline_edit = None;
                if let Some(service) = self.service.clone() {
                    let kind = MutationKind::ChangeTags {
                        id: task_id.clone(),
                        tags: tokens.clone(),
                    };
                    self.apply_optimistic_update(&[task_id.clone()], &kind);
                    self.pending_mutations += 1;
                    return Effect::perform(
                        mutation_command(service, kind.clone()),
                        move |result| Message::MutationFinished(kind.clone(), result),
                    );
                } else {
                    self.apply_optimistic_tags(&task_id, &tokens);
                    return Effect::none();
                }
            }
            InlineEditableField::Priority => {
                let Some(new_priority) = priority_from_input(&edit.value) else {
                    self.status = Some(StatusToast {
                        message: "Priority must be between 0 (none) and 3 (high).".into(),
                        kind: ToastKind::Error,
                        created_at: Instant::now(),
                    });
                    return Effect::none();
                };

                let current_priority = self
                    .current_tasks()
                    .into_iter()
                    .find(|task| task.id == edit.task_id)
                    .map(|task| task.priority)
                    .unwrap_or_default();

                if new_priority == current_priority {
                    self.inline_edit = None;
                    return Effect::none();
                }

                let task_id = edit.task_id.clone();
                self.inline_edit = None;
                if let Some(service) = self.service.clone() {
                    let kind = MutationKind::ChangePriority {
                        id: task_id.clone(),
                        priority: new_priority,
                    };
                    self.apply_optimistic_update(&[task_id.clone()], &kind);
                    self.pending_mutations += 1;
                    return Effect::perform(
                        mutation_command(service, kind.clone()),
                        move |result| Message::MutationFinished(kind.clone(), result),
                    );
                } else {
                    self.apply_optimistic_priority(&task_id, new_priority);
                    return Effect::none();
                }
            }
        }
    }

    pub(super) fn finish_capture(&mut self, result: Result<AddOutcome, String>) -> Effect {
        self.capture.submitting = false;
        match result {
            Ok(outcome) => {
                self.status = Some(StatusToast {
                    message: format!("Added task '{}'.", outcome.title),
                    kind: ToastKind::Info,
                    created_at: Instant::now(),
                });
                self.telemetry
                    .record(TelemetryEvent::CaptureFinished(outcome.id));
                self.capture.clear();
                self.capture.open = false;
                self.refresh_active_view()
            }
            Err(err) => {
                self.capture.preview_error = Some(err.clone());
                self.status = Some(StatusToast {
                    message: err,
                    kind: ToastKind::Error,
                    created_at: Instant::now(),
                });
                Effect::none()
            }
        }
    }

    pub(super) fn finish_mutation(
        &mut self,
        kind: MutationKind,
        result: Result<(), String>,
    ) -> Effect {
        self.pending_mutations = self.pending_mutations.saturating_sub(1);
        match result {
            Ok(()) => {
                self.status = Some(StatusToast {
                    message: format!("{} succeeded", capitalize(kind.label())),
                    kind: ToastKind::Info,
                    created_at: Instant::now(),
                });
                self.telemetry
                    .record(TelemetryEvent::MutationApplied(kind.label().into()));
                self.refresh_active_view()
            }
            Err(err) => {
                self.status = Some(StatusToast {
                    message: err.clone(),
                    kind: ToastKind::Error,
                    created_at: Instant::now(),
                });
                self.telemetry.record(TelemetryEvent::MutationFailed {
                    action: kind.label().into(),
                    error: err,
                });
                Effect::none()
            }
        }
    }

    pub(super) fn focus_capture_input(&self) -> Effect {
        Effect::batch(vec![
            focus(self.capture_input_id.clone()),
            move_cursor_to_end(self.capture_input_id.clone()),
        ])
    }

    pub(super) fn submit_capture(&mut self) -> Effect {
        if !self.capture.open || self.capture.submitting {
            return Effect::none();
        }
        if self.capture.text.trim().is_empty() {
            self.capture.preview_error = Some("Task text cannot be empty".into());
            return Effect::none();
        }
        if let Some(service) = self.service.clone() {
            self.capture.submitting = true;
            self.telemetry.record(TelemetryEvent::CaptureStarted);
            let input = self.capture.input();
            Effect::perform(capture_command(service, input), Message::CaptureCompleted)
        } else {
            Effect::none()
        }
    }

    pub(super) fn refresh_active_view(&mut self) -> Effect {
        if let Some(service) = self.service.clone() {
            self.telemetry
                .record(TelemetryEvent::RefreshRequested(self.active.title().into()));
            self.views
                .entry(self.active)
                .and_modify(|view| view.state = LoadState::Loading);
            load_view_command(service, self.active)
        } else {
            Effect::none()
        }
    }

    pub(super) fn switch_view(&mut self, tab: ViewTab) -> Effect {
        self.active = tab;
        self.ensure_view_entry(tab);
        self.telemetry
            .record(TelemetryEvent::ViewChanged(tab.title().into()));
        self.telemetry
            .record(TelemetryEvent::RefreshRequested(tab.title().into()));
        if let Some(store) = self.views.get_mut(&tab) {
            store.state = LoadState::Loading;
        }
        if let Some(service) = self.service.clone() {
            load_view_command(service, tab)
        } else {
            Effect::none()
        }
    }

    pub(super) fn sync_selection_with_view(&mut self) {
        let selected_id = self.selected_task.clone();
        let tasks = self.current_tasks();
        let has_selected = selected_id
            .as_ref()
            .and_then(|id| tasks.iter().position(|task| &task.id == id))
            .is_some();
        if !has_selected {
            self.selected_task = tasks.first().map(|task| task.id.clone());
        }
    }

    pub(super) fn handle_keyboard(&mut self, event: KeyboardEvent) -> Effect {
        match event {
            KeyboardEvent::KeyPressed { key, modifiers, .. } => {
                if modifiers.command() {
                    if let Key::Character(value) = key.as_ref() {
                        if value.eq_ignore_ascii_case("k") {
                            return self.toggle_command_palette();
                        }
                    }
                }

                if self.inline_edit.is_some() {
                    match key.as_ref() {
                        Key::Named(Named::Escape) => {
                            self.cancel_inline_edit();
                        }
                        Key::Named(Named::Enter) => return self.submit_inline_edit(),
                        _ => {}
                    }
                    return Effect::none();
                }

                if self.command_palette.open {
                    match key.as_ref() {
                        Key::Named(Named::ArrowDown) => {
                            self.command_palette.move_selection(1);
                        }
                        Key::Named(Named::ArrowUp) => {
                            self.command_palette.move_selection(-1);
                        }
                        Key::Named(Named::Escape) => {
                            self.command_palette.close();
                        }
                        Key::Named(Named::Enter) => {
                            if let Some(action) = self.command_palette.selected_action() {
                                let id = action.id;
                                return self.handle_action(id);
                            }
                        }
                        _ => {}
                    }
                    return Effect::none();
                }

                if self.capture.open {
                    match key.as_ref() {
                        Key::Named(Named::Escape) => {
                            self.capture.clear();
                            self.capture.open = false;
                        }
                        Key::Named(Named::Enter) => return self.submit_capture(),
                        _ => {}
                    }
                    return Effect::none();
                }

                match key.as_ref() {
                    Key::Named(Named::Tab) => {
                        if modifiers.shift() {
                            return self.move_tab(-1);
                        } else {
                            return self.move_tab(1);
                        }
                    }
                    Key::Named(Named::ArrowDown) => {
                        self.move_selection(1);
                        Effect::none()
                    }
                    Key::Named(Named::ArrowUp) => {
                        self.move_selection(-1);
                        Effect::none()
                    }
                    Key::Character(value) => match value.to_ascii_lowercase().as_str() {
                        "/" | "?" => self.toggle_command_palette(),
                        "a" => self.toggle_capture(),
                        "d" => self.handle_action(CommandActionId::MarkDone),
                        "n" => self.handle_action(CommandActionId::PromoteNext),
                        "i" => self.handle_action(CommandActionId::MoveToInbox),
                        "r" => self.refresh_active_view(),
                        _ => Effect::none(),
                    },
                    _ => Effect::none(),
                }
            }
            _ => Effect::none(),
        }
    }

    pub(super) fn move_tab(&mut self, delta: i32) -> Effect {
        let tabs = ViewTab::ALL;
        let len = tabs.len() as i32;
        let current_index = tabs.iter().position(|tab| *tab == self.active).unwrap_or(0) as i32;
        let mut next = current_index + delta;
        if next < 0 {
            next = len - 1;
        } else if next >= len {
            next = 0;
        }
        let tab = tabs[next as usize];
        self.switch_view(tab)
    }

    pub(super) fn move_selection(&mut self, delta: i32) {
        let tasks = self.current_tasks();
        if tasks.is_empty() {
            self.selected_task = None;
            return;
        }
        let current_index = self
            .selected_task
            .as_ref()
            .and_then(|id| tasks.iter().position(|task| &task.id == id))
            .unwrap_or(0);
        let len = tasks.len() as i32;
        let mut next = current_index as i32 + delta;
        if next < 0 {
            next = len - 1;
        } else if next >= len {
            next = 0;
        }
        self.selected_task = Some(tasks[next as usize].id.clone());
    }

    pub(super) fn handle_action(&mut self, action: CommandActionId) -> Effect {
        match action {
            CommandActionId::OpenCapture => {
                if self.capture.open {
                    self.focus_capture_input()
                } else {
                    self.toggle_capture()
                }
            }
            CommandActionId::Refresh => self.refresh_active_view(),
            CommandActionId::PromoteNext => {
                if let Some(ids) = self.selected_ids() {
                    self.apply_status_change(ids, MutationKind::Promote)
                } else {
                    Effect::none()
                }
            }
            CommandActionId::MarkDone => {
                if let Some(ids) = self.selected_ids() {
                    self.apply_status_change(ids, MutationKind::Complete)
                } else {
                    Effect::none()
                }
            }
            CommandActionId::MoveToInbox => {
                if let Some(ids) = self.selected_ids() {
                    self.apply_status_change(ids, MutationKind::Inbox)
                } else {
                    Effect::none()
                }
            }
            CommandActionId::DeferTomorrow => self.defer_selected(ChronoDuration::days(1)),
            CommandActionId::DeferNextWeek => self.defer_selected(ChronoDuration::days(7)),
        }
    }

    pub(super) fn selected_ids(&self) -> Option<Vec<String>> {
        let selected = self.selected_task.as_ref()?;
        Some(vec![selected.clone()])
    }

    pub(super) fn apply_status_change(
        &mut self,
        ids: Vec<String>,
        constructor: impl Fn(Vec<String>) -> MutationKind,
    ) -> Effect {
        if let Some(service) = self.service.clone() {
            let kind = constructor(ids.clone());
            self.apply_optimistic_update(&ids, &kind);
            self.pending_mutations += 1;
            Effect::perform(mutation_command(service, kind.clone()), move |result| {
                Message::MutationFinished(kind.clone(), result)
            })
        } else {
            Effect::none()
        }
    }

    pub(super) fn defer_selected(&mut self, offset: ChronoDuration) -> Effect {
        let Some(selected) = self.selected_task.clone() else {
            return Effect::none();
        };
        let until = Utc::now() + offset;
        if let Some(service) = self.service.clone() {
            let kind = MutationKind::Defer {
                id: selected.clone(),
                until,
            };
            self.apply_optimistic_defer(&selected, until);
            self.pending_mutations += 1;
            Effect::perform(mutation_command(service, kind.clone()), move |result| {
                Message::MutationFinished(kind.clone(), result)
            })
        } else {
            Effect::none()
        }
    }

    pub(super) fn apply_optimistic_update(&mut self, ids: &[String], kind: &MutationKind) {
        match kind {
            MutationKind::Rename { id, title } => self.apply_optimistic_title(id, title),
            MutationKind::ChangeProject { id, project } => {
                self.apply_optimistic_project(id, project.clone())
            }
            MutationKind::ChangeContexts { id, contexts } => {
                self.apply_optimistic_contexts(id, contexts)
            }
            MutationKind::ChangeTags { id, tags } => self.apply_optimistic_tags(id, tags),
            MutationKind::ChangePriority { id, priority } => {
                self.apply_optimistic_priority(id, *priority)
            }
            _ => {
                if let Some(store) = self.views.get_mut(&self.active) {
                    if let Some(snapshot) = store.snapshot.as_mut() {
                        let should_remove = match kind {
                            MutationKind::Promote(_) => self.active != ViewTab::Next,
                            MutationKind::Complete(_) => true,
                            MutationKind::Inbox(_) => self.active != ViewTab::Inbox,
                            MutationKind::Defer { .. } => false,
                            _ => false,
                        };
                        if should_remove {
                            snapshot.tasks.retain(|task| !ids.contains(&task.id));
                        }
                        store.version = store.version.wrapping_add(1);
                    }
                }
            }
        }
        self.sync_selection_with_view();
        let label = kind.label();
        self.status = Some(StatusToast {
            message: format!("Queued {label}…"),
            kind: ToastKind::Info,
            created_at: Instant::now(),
        });
    }

    fn apply_optimistic_title(&mut self, id: &str, title: &str) {
        if let Some(store) = self.views.get_mut(&self.active) {
            if let Some(snapshot) = store.snapshot.as_mut() {
                if let Some(task) = snapshot.tasks.iter_mut().find(|task| task.id == id) {
                    task.title = title.to_string();
                }
                store.version = store.version.wrapping_add(1);
            }
        }
    }

    fn apply_optimistic_project(&mut self, id: &str, project: Option<String>) {
        if let Some(store) = self.views.get_mut(&self.active) {
            if let Some(snapshot) = store.snapshot.as_mut() {
                if let Some(task) = snapshot.tasks.iter_mut().find(|task| task.id == id) {
                    task.project = project;
                }
                store.version = store.version.wrapping_add(1);
            }
        }
    }

    fn apply_optimistic_contexts(&mut self, id: &str, contexts: &[String]) {
        if let Some(store) = self.views.get_mut(&self.active) {
            if let Some(snapshot) = store.snapshot.as_mut() {
                if let Some(task) = snapshot.tasks.iter_mut().find(|task| task.id == id) {
                    task.contexts = contexts.to_vec();
                }
                store.version = store.version.wrapping_add(1);
            }
        }
    }

    fn apply_optimistic_tags(&mut self, id: &str, tags: &[String]) {
        if let Some(store) = self.views.get_mut(&self.active) {
            if let Some(snapshot) = store.snapshot.as_mut() {
                if let Some(task) = snapshot.tasks.iter_mut().find(|task| task.id == id) {
                    task.tags = tags.to_vec();
                }
                store.version = store.version.wrapping_add(1);
            }
        }
    }

    fn apply_optimistic_priority(&mut self, id: &str, priority: u8) {
        if let Some(store) = self.views.get_mut(&self.active) {
            if let Some(snapshot) = store.snapshot.as_mut() {
                if let Some(task) = snapshot.tasks.iter_mut().find(|task| task.id == id) {
                    task.priority = priority;
                }
                store.version = store.version.wrapping_add(1);
            }
        }
    }

    pub(super) fn apply_optimistic_defer(&mut self, id: &str, until: DateTime<Utc>) {
        if let Some(store) = self.views.get_mut(&self.active) {
            if let Some(snapshot) = store.snapshot.as_mut() {
                if let Some(task) = snapshot.tasks.iter_mut().find(|task| task.id == id) {
                    task.defer_until = Some(until);
                    if matches!(self.active, ViewTab::Inbox | ViewTab::Next) {
                        task.status = TaskStatus::Scheduled;
                    }
                }
                snapshot
                    .tasks
                    .retain(|task| task.id != id || matches!(self.active, ViewTab::Scheduled));
                store.version = store.version.wrapping_add(1);
            }
        }
        self.sync_selection_with_view();
        self.status = Some(StatusToast {
            message: "Queued defer…".into(),
            kind: ToastKind::Info,
            created_at: Instant::now(),
        });
    }

    pub(super) fn current_tasks(&self) -> Vec<&Task> {
        self.views
            .get(&self.active)
            .and_then(|view| view.snapshot.as_ref())
            .map(|snapshot| snapshot.tasks.iter().collect())
            .unwrap_or_default()
    }
}

impl CptDesktop {
    fn collect_projects(&self) -> Vec<String> {
        let mut set = BTreeSet::new();
        for task in self.current_tasks() {
            if let Some(project) = task.project.as_ref() {
                let trimmed = project.trim();
                if !trimmed.is_empty() {
                    set.insert(trimmed.to_string());
                }
            }
        }
        set.into_iter().collect()
    }

    fn collect_contexts(&self) -> Vec<String> {
        let mut set = BTreeSet::new();
        for task in self.current_tasks() {
            for ctx in task.contexts.iter() {
                let trimmed = ctx.trim();
                if !trimmed.is_empty() {
                    set.insert(trimmed.to_string());
                }
            }
        }
        set.into_iter().collect()
    }

    fn collect_tags(&self) -> Vec<String> {
        let mut set = BTreeSet::new();
        for task in self.current_tasks() {
            for tag in task.tags.iter() {
                let trimmed = tag.trim();
                if !trimmed.is_empty() {
                    set.insert(trimmed.to_string());
                }
            }
        }
        set.into_iter().collect()
    }
}

fn priority_options() -> Vec<String> {
    PRIORITY_CHOICES
        .iter()
        .map(|(value, label)| format!("{label} ({value})"))
        .collect()
}

fn priority_label(priority: u8) -> String {
    PRIORITY_CHOICES
        .iter()
        .find(|(value, _)| *value == priority)
        .map(|(value, label)| format!("{label} ({value})"))
        .unwrap_or_else(|| format!("None ({priority})"))
}

fn priority_from_input(value: &str) -> Option<u8> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Some(0);
    }
    let lowered = trimmed.to_ascii_lowercase();

    for (priority, label) in PRIORITY_CHOICES.iter() {
        let label_lower = label.to_ascii_lowercase();
        if lowered.contains(&label_lower) {
            return Some(*priority);
        }
    }

    for ch in lowered.chars() {
        if let Some(digit) = ch.to_digit(10) {
            if digit <= 3 {
                return Some(digit as u8);
            }
        }
    }

    if lowered.starts_with('p') {
        if let Some(ch) = lowered.chars().skip(1).find(|c| c.is_ascii_digit()) {
            if let Some(digit) = ch.to_digit(10) {
                if digit <= 3 {
                    return Some(digit as u8);
                }
            }
        }
    }

    None
}

fn parse_token_list(value: &str) -> Vec<String> {
    let use_commas = value.contains(',') || value.contains(';');
    let iter: Box<dyn Iterator<Item = &str>> = if use_commas {
        Box::new(value.split(|c| c == ',' || c == ';'))
    } else {
        Box::new(value.split_whitespace())
    };

    let mut seen = Vec::new();
    let mut result = Vec::new();
    for token in iter {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            continue;
        }
        let key = trimmed.to_ascii_lowercase();
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        result.push(trimmed.to_string());
    }
    result
}
