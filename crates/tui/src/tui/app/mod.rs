use std::time::{Duration, Instant};

use anyhow::Result;
use ratatui::style::{Color, Style};
use ratatui::widgets::TableState;

use super::buffer::TextBuffer;
use super::constants::*;
use super::filters::{ActiveFilters, FilterFacets, FilterOverlay};
use super::helpers::compose_task_capture;
use crate::capture::CaptureInput;
use crate::config::AppConfig;
use crate::db::Database;
use crate::model::{ListFilters, ListOutputItem, ListView, ProjectSummary, Task, TaskStatus};
use crate::parser;

mod commands;
mod input;
mod render;
#[cfg(test)]
mod tests;

use commands::Suggestion;

#[derive(Debug, Clone)]
struct ViewTab {
    label: &'static str,
    view: Option<ListView>,
    description: &'static str,
}

impl ViewTab {
    pub(crate) fn new(
        label: &'static str,
        view: Option<ListView>,
        description: &'static str,
    ) -> Self {
        Self {
            label,
            view,
            description,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputMode {
    Normal,
    Add,
    Command,
    Filter,
    Edit,
    Inspect,
    Help,
    ConfirmDelete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConfirmChoice {
    Yes,
    No,
}

impl ConfirmChoice {
    fn toggle(self) -> Self {
        match self {
            ConfirmChoice::Yes => ConfirmChoice::No,
            ConfirmChoice::No => ConfirmChoice::Yes,
        }
    }
}

#[derive(Debug, Clone)]
struct StatusMessage {
    text: String,
    kind: StatusKind,
    created_at: Instant,
}

impl StatusMessage {
    fn new<T: Into<String>>(text: T, kind: StatusKind) -> Self {
        Self {
            text: text.into(),
            kind,
            created_at: Instant::now(),
        }
    }

    fn style(&self) -> Style {
        match self.kind {
            StatusKind::Info => Style::default().fg(Color::Cyan),
            StatusKind::Error => Style::default().fg(Color::Red),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum StatusKind {
    Info,
    Error,
}

pub(crate) struct App {
    config: AppConfig,
    database: Database,
    first_run: bool,
    tabs: Vec<ViewTab>,
    tab_index: usize,
    tasks: Vec<Task>,
    projects: Vec<ProjectSummary>,
    showing_projects: bool,
    selected: usize,
    table_state: TableState,
    input_mode: InputMode,
    input: TextBuffer,
    suggestions: Vec<Suggestion>,
    suggestion_index: usize,
    status: Option<StatusMessage>,
    active_filters: ActiveFilters,
    filter_overlay: Option<FilterOverlay>,
    editing_task_id: Option<String>,
    inspect_task: Option<Task>,
    confirm_choice: ConfirmChoice,
    should_quit: bool,
}

impl App {
    pub(crate) fn new(config: AppConfig, database: Database, first_run: bool) -> Result<Self> {
        let tabs = vec![
            ViewTab::new("📋 All", None, "All active tasks"),
            ViewTab::new("📥 Inbox", Some(ListView::Inbox), "Inbox items"),
            ViewTab::new("⚡ Next", Some(ListView::Next), "Next actions"),
            ViewTab::new("⏳ Waiting", Some(ListView::Waiting), "Waiting on others"),
            ViewTab::new("📅 Scheduled", Some(ListView::Scheduled), "Scheduled work"),
            ViewTab::new("🌱 Someday", Some(ListView::Someday), "Someday/Maybe"),
            ViewTab::new("📂 Projects", Some(ListView::Projects), "Project health"),
            ViewTab::new("✅ Done", Some(ListView::Done), "Completed tasks"),
        ];

        let mut app = Self {
            config,
            database,
            first_run,
            tabs,
            tab_index: 0,
            tasks: Vec::new(),
            projects: Vec::new(),
            showing_projects: false,
            selected: 0,
            table_state: TableState::default(),
            input_mode: InputMode::Normal,
            input: TextBuffer::new(),
            suggestions: Vec::new(),
            suggestion_index: 0,
            status: None,
            active_filters: ActiveFilters::default(),
            filter_overlay: None,
            editing_task_id: None,
            inspect_task: None,
            confirm_choice: ConfirmChoice::No,
            should_quit: false,
        };
        app.refresh()?;
        Ok(app)
    }

    fn current_view(&self) -> Option<ListView> {
        self.tabs
            .get(self.tab_index)
            .and_then(|tab| tab.view.clone())
    }

    pub(crate) fn refresh(&mut self) -> Result<()> {
        let mut filters = ListFilters::for_view(self.current_view());
        self.active_filters.apply_to(&mut filters);
        let items = self.database.fetch_tasks(&filters)?;
        self.tasks.clear();
        self.projects.clear();
        self.showing_projects = matches!(self.current_view(), Some(ListView::Projects));
        for item in items {
            match item {
                ListOutputItem::Task(task) => self.tasks.push(*task),
                ListOutputItem::Project(project) => self.projects.push(project),
            }
        }

        let had_items = !self.tasks.is_empty() || !self.projects.is_empty();

        if self.first_run && had_items {
            self.first_run = false;
        }

        if self.tasks.is_empty() {
            self.selected = 0;
            self.table_state.select(None);
        } else {
            if self.selected >= self.tasks.len() {
                self.selected = self.tasks.len() - 1;
            }
            self.table_state.select(Some(self.selected));
        }

        Ok(())
    }

    pub(crate) fn on_tick(&mut self) {
        if let Some(status) = &self.status {
            if status.created_at.elapsed() > Duration::from_secs(5) {
                self.status = None;
            }
        }
    }

    pub(crate) fn should_quit(&self) -> bool {
        self.should_quit
    }

    fn ensure_task_view(&mut self, message: &str) -> bool {
        if self.showing_projects {
            self.set_status_info(message);
            false
        } else {
            true
        }
    }

    fn select_next(&mut self) {
        if self.tasks.is_empty() {
            return;
        }
        self.selected = (self.selected + 1).min(self.tasks.len() - 1);
        self.table_state.select(Some(self.selected));
    }

    fn select_prev(&mut self) {
        if self.tasks.is_empty() {
            return;
        }
        if self.selected > 0 {
            self.selected -= 1;
        }
        self.table_state.select(Some(self.selected));
    }

    fn select_task_by_id(&mut self, id: &str) {
        if let Some((idx, _)) = self
            .tasks
            .iter()
            .enumerate()
            .find(|(_, task)| task.id == id)
        {
            self.selected = idx;
            self.table_state.select(Some(idx));
        }
    }

    fn next_tab(&mut self) -> Result<()> {
        self.tab_index = (self.tab_index + 1) % self.tabs.len();
        self.refresh()
    }

    fn prev_tab(&mut self) -> Result<()> {
        if self.tab_index == 0 {
            self.tab_index = self.tabs.len() - 1;
        } else {
            self.tab_index -= 1;
        }
        self.refresh()
    }

    fn start_edit_current(&mut self) -> Result<()> {
        if !self.ensure_task_view(STATUS_PROJECT_EDIT) {
            return Ok(());
        }
        if self.tasks.is_empty() {
            self.set_status_info("Nothing to edit");
            return Ok(());
        }

        let task = self.tasks[self.selected].clone();
        self.begin_edit_with_task(task);
        Ok(())
    }

    fn start_edit_by_id(&mut self, id: String) -> Result<()> {
        if let Some((idx, task)) = self
            .tasks
            .iter()
            .enumerate()
            .find(|(_, task)| task.id == id)
        {
            self.selected = idx;
            self.table_state.select(Some(idx));
            self.begin_edit_with_task(task.clone());
            return Ok(());
        }

        match self.database.fetch_task(&id)? {
            Some(task) => {
                self.begin_edit_with_task(task);
                Ok(())
            }
            None => {
                self.set_status_error("Task not found");
                Ok(())
            }
        }
    }

    fn begin_edit_with_task(&mut self, task: Task) {
        self.input.set(compose_task_capture(&task));
        self.input_mode = InputMode::Edit;
        self.editing_task_id = Some(task.id);
        self.set_status_info(STATUS_ENTER_EDIT);
    }

    fn open_filter_overlay(&mut self) -> Result<()> {
        if self.showing_projects {
            self.set_status_info("Filters are not available on the Projects summary");
            return Ok(());
        }

        let base_filters = ListFilters::for_view(self.current_view());
        let items = self.database.fetch_tasks(&base_filters)?;
        let tasks: Vec<Task> = items
            .into_iter()
            .filter_map(|item| match item {
                ListOutputItem::Task(task) => Some(*task),
                _ => None,
            })
            .collect();

        let facets = FilterFacets::from_tasks(&tasks);
        self.filter_overlay = Some(FilterOverlay::new(facets, &self.active_filters));
        self.input_mode = InputMode::Filter;
        self.set_status_info(STATUS_FILTER_PICKER);
        Ok(())
    }

    fn add_task(&mut self) -> Result<()> {
        let parts: Vec<String> = self
            .input
            .as_str()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        if parts.is_empty() {
            self.set_status_error("Enter some details before capturing a task");
            return Ok(());
        }

        let capture = CaptureInput {
            text: parts,
            notes: None,
            project: None,
            areas: Vec::new(),
            status: None,
            contexts: Vec::new(),
            tags: Vec::new(),
            due_at: None,
            defer_until: None,
            time_estimate: None,
            energy: None,
            priority: None,
            waiting_on: None,
            waiting_since: None,
        };

        let outcome = self.database.handle_add(&capture)?;
        self.set_status_info(format!(
            "Captured [{}] {}",
            outcome.status.as_str(),
            outcome.title
        ));
        self.input.clear();
        self.input_mode = InputMode::Normal;
        self.refresh()?;
        Ok(())
    }

    fn mark_next(&mut self) -> Result<()> {
        if self.tasks.is_empty() {
            self.set_status_info("Nothing to mark next");
            return Ok(());
        }
        let id = self.tasks[self.selected].id.clone();
        let results = self.database.mark_next(&[id.clone()])?;
        if results.iter().any(|r| r.changed) {
            self.set_status_info("Moved task to next actions");
        } else {
            self.set_status_info("Task already in next actions");
        }
        self.refresh()?;
        Ok(())
    }

    fn mark_someday(&mut self) -> Result<()> {
        if self.tasks.is_empty() {
            self.set_status_info("Nothing to move to Someday");
            return Ok(());
        }
        let id = self.tasks[self.selected].id.clone();
        let results = self.database.mark_someday(&[id.clone()])?;
        if results.iter().any(|r| r.changed) {
            self.set_status_info("Moved task to Someday/Maybe");
        } else {
            self.set_status_info("Task already in Someday");
        }
        self.refresh()?;
        Ok(())
    }

    fn mark_inbox(&mut self) -> Result<()> {
        if self.tasks.is_empty() {
            self.set_status_info("Nothing to move to Inbox");
            return Ok(());
        }
        let id = self.tasks[self.selected].id.clone();
        let results = self.database.mark_inbox(&[id.clone()])?;
        if results.iter().any(|r| r.changed) {
            self.set_status_info("Moved task back to Inbox");
        } else {
            self.set_status_info("Task already in Inbox");
        }
        self.refresh()?;
        Ok(())
    }

    fn show_selected_details(&mut self) -> Result<()> {
        if self.showing_projects {
            self.set_status_info("Task details unavailable in Projects view");
            return Ok(());
        }
        if self.tasks.is_empty() {
            self.set_status_info("Nothing to inspect");
            return Ok(());
        }

        let task = self.tasks[self.selected].clone();
        self.inspect_task = Some(task);
        self.input_mode = InputMode::Inspect;
        self.set_status_info(STATUS_VIEW_DETAILS);
        Ok(())
    }

    fn show_help_overlay(&mut self) {
        self.inspect_task = None;
        self.input_mode = InputMode::Help;
        self.set_status_info(STATUS_HELP);
    }

    fn prompt_delete(&mut self) {
        if self.tasks.is_empty() {
            self.set_status_info("Nothing to delete");
            return;
        }
        self.confirm_choice = ConfirmChoice::No;
        self.input_mode = InputMode::ConfirmDelete;
        self.set_status_info(STATUS_CONFIRM_DELETE);
    }

    fn mark_done(&mut self) -> Result<()> {
        if self.tasks.is_empty() {
            self.set_status_info("Nothing to mark done");
            return Ok(());
        }
        let id = self.tasks[self.selected].id.clone();
        let results = self.database.mark_done(&[id.clone()])?;
        if results.iter().any(|r| r.changed) {
            self.set_status_info("Marked task as done");
        } else {
            self.set_status_info("Task was already done");
        }
        self.refresh()?;
        Ok(())
    }

    fn apply_edit(&mut self) -> Result<()> {
        let id = match self.editing_task_id.clone() {
            Some(id) => id,
            None => {
                self.set_status_error("No task selected for editing");
                return Ok(());
            }
        };

        let text = self.input.as_str().trim().to_string();
        if text.is_empty() {
            self.set_status_error("Enter some details before saving");
            return Ok(());
        }

        self.edit_task_with_text(id, &text)?;
        self.input.clear();
        self.editing_task_id = None;
        self.input_mode = InputMode::Normal;
        Ok(())
    }

    fn cancel_edit(&mut self) {
        self.editing_task_id = None;
        self.input.clear();
        self.input_mode = InputMode::Normal;
        self.status = None;
    }

    fn edit_task_with_text(&mut self, id: String, text: &str) -> Result<()> {
        let tokens: Vec<String> = text.split_whitespace().map(|s| s.to_string()).collect();

        if tokens.is_empty() {
            self.set_status_error("Enter some details before saving");
            return Ok(());
        }

        let capture = CaptureInput {
            text: tokens,
            notes: None,
            project: None,
            areas: Vec::new(),
            status: None,
            contexts: Vec::new(),
            tags: Vec::new(),
            due_at: None,
            defer_until: None,
            time_estimate: None,
            energy: None,
            priority: None,
            waiting_on: None,
            waiting_since: None,
        };

        let parsed = match parser::parse_capture(&capture) {
            Ok(parsed) => parsed,
            Err(err) => {
                self.set_status_error(format!("Edit failed: {}", err));
                return Ok(());
            }
        };

        let existing = match self.database.fetch_task(&id)? {
            Some(task) => task,
            None => {
                self.set_status_error("Task not found");
                return Ok(());
            }
        };

        let mut updated_task = parsed.task;
        if updated_task.notes.is_none() {
            updated_task.notes = existing.notes.clone();
        }
        if updated_task.areas.is_empty() {
            updated_task.areas = existing.areas.clone();
        }
        if updated_task.repeat.is_none() {
            updated_task.repeat = existing.repeat.clone();
        }

        let mut final_status = existing.status;
        if updated_task.waiting_on.is_some() {
            final_status = TaskStatus::Waiting;
        } else if matches!(existing.status, TaskStatus::Waiting)
            && updated_task.waiting_on.is_none()
        {
            final_status = TaskStatus::Inbox;
        }
        updated_task.status = final_status;

        let updated = match self.database.update_task(&id, &updated_task)? {
            Some(task) => task,
            None => {
                self.set_status_error("Task not found");
                return Ok(());
            }
        };

        self.refresh()?;
        self.select_task_by_id(&id);
        self.set_status_info(format!(
            "Updated [{}] {}",
            updated.status.as_str(),
            updated.title
        ));
        Ok(())
    }

    fn perform_delete(&mut self) -> Result<()> {
        if self.tasks.is_empty() {
            self.set_status_info("Nothing to delete");
            return Ok(());
        }
        let id = self.tasks[self.selected].id.clone();
        let results = self.database.delete_tasks(&[id.clone()])?;
        if results.iter().any(|r| r.deleted) {
            self.set_status_info("Deleted task 🗑️");
        } else {
            self.set_status_info("Task not found");
        }
        self.refresh()?;
        Ok(())
    }

    pub(crate) fn set_status_info<T: Into<String>>(&mut self, message: T) {
        let mut text = String::from("ℹ️  ");
        text.push_str(&message.into());
        self.status = Some(StatusMessage::new(text, StatusKind::Info));
    }

    pub(crate) fn set_status_error<T: Into<String>>(&mut self, message: T) {
        let mut text = String::from("⚠️  ");
        text.push_str(&message.into());
        self.status = Some(StatusMessage::new(text, StatusKind::Error));
    }
}
