//! Shared state models that keep the desktop UI in sync with cpt.run tasks.

use std::time::Instant;

use chrono::{DateTime, Utc};
use cpt_core::capture::CaptureInput;
use cpt_core::model::{ListView, TaskStatus};
use cpt_core::ViewSnapshot;
use iced::widget::Id;

use crate::app::helpers::capture_preview;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ViewTab {
    All,
    Inbox,
    Next,
    Waiting,
    Scheduled,
    Projects,
}

impl ViewTab {
    pub(crate) const ALL: &'static [ViewTab] = &[
        ViewTab::All,
        ViewTab::Inbox,
        ViewTab::Next,
        ViewTab::Scheduled,
        ViewTab::Waiting,
        ViewTab::Projects,
    ];

    pub(crate) fn title(self) -> &'static str {
        match self {
            ViewTab::All => "All",
            ViewTab::Inbox => "Inbox",
            ViewTab::Next => "Next",
            ViewTab::Waiting => "Waiting",
            ViewTab::Scheduled => "Scheduled",
            ViewTab::Projects => "Projects",
        }
    }

    pub(crate) fn subtitle(self) -> &'static str {
        match self {
            ViewTab::All => "Unified task inventory",
            ViewTab::Inbox => "Collect everything, triage quickly",
            ViewTab::Next => "High-signal next actions ready for focus",
            ViewTab::Waiting => "People & dependencies to follow up",
            ViewTab::Scheduled => "Deferred or time-specific commitments",
            ViewTab::Projects => "See projects with next steps",
        }
    }

    pub(crate) fn list_view(self) -> Option<ListView> {
        match self {
            ViewTab::All => None,
            ViewTab::Inbox => Some(ListView::Inbox),
            ViewTab::Next => Some(ListView::Next),
            ViewTab::Waiting => Some(ListView::Waiting),
            ViewTab::Scheduled => Some(ListView::Scheduled),
            ViewTab::Projects => Some(ListView::Projects),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ViewStore {
    pub(crate) snapshot: Option<ViewSnapshot>,
    pub(crate) state: LoadState,
    pub(crate) version: u64,
    pub(crate) last_refreshed: Option<Instant>,
}

impl ViewStore {
    pub(crate) fn new() -> Self {
        Self {
            snapshot: None,
            state: LoadState::Idle,
            version: 0,
            last_refreshed: None,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum LoadState {
    Idle,
    Loading,
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum InlineEditableField {
    Title,
    Project,
    Contexts,
    Tags,
    Priority,
}

#[derive(Clone)]
pub(crate) struct InlineEditState {
    pub(crate) task_id: String,
    pub(crate) field: InlineEditableField,
    pub(crate) value: String,
    pub(crate) original_value: String,
    pub(crate) input_id: Id,
    pub(crate) options: Vec<String>,
    pub(crate) original_tokens: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct StatusToast {
    pub(crate) message: String,
    pub(crate) kind: ToastKind,
    pub(crate) created_at: Instant,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ToastKind {
    Info,
    Error,
}

#[derive(Debug, Clone)]
pub(crate) struct CaptureState {
    pub(crate) open: bool,
    pub(crate) text: String,
    pub(crate) preview: Option<CapturePreview>,
    pub(crate) preview_error: Option<String>,
    pub(crate) submitting: bool,
}

impl CaptureState {
    pub(crate) fn new() -> Self {
        Self {
            open: false,
            text: String::new(),
            preview: None,
            preview_error: None,
            submitting: false,
        }
    }

    pub(crate) fn toggle(&mut self) {
        self.open = !self.open;
        if !self.open {
            self.clear();
        }
    }

    pub(crate) fn clear(&mut self) {
        self.text.clear();
        self.preview = None;
        self.preview_error = None;
        self.submitting = false;
    }

    pub(crate) fn on_text_changed(&mut self, value: String) {
        self.text = value;
        if self.text.trim().is_empty() {
            self.preview = None;
            self.preview_error = None;
            return;
        }

        match capture_preview(&self.text) {
            Ok(preview) => {
                self.preview = preview;
                self.preview_error = None;
            }
            Err(err) => {
                self.preview = None;
                self.preview_error = Some(err);
            }
        }
    }

    pub(crate) fn input(&self) -> CaptureInput {
        CaptureInput {
            text: self
                .text
                .split_whitespace()
                .map(|piece| piece.to_string())
                .collect(),
            ..CaptureInput::default()
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CapturePreview {
    pub(crate) chips: Vec<CaptureChip>,
}

#[derive(Debug, Clone)]
pub(crate) struct CaptureChip {
    pub(crate) label: String,
    pub(crate) kind: CaptureChipKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CaptureChipKind {
    Project,
    Context,
    Tag,
    Due,
    Defer,
    Energy,
    Priority,
    Waiting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandActionId {
    OpenCapture,
    PromoteNext,
    MarkDone,
    MoveToInbox,
    DeferTomorrow,
    DeferNextWeek,
    Refresh,
}

#[derive(Debug, Clone)]
pub(crate) struct CommandAction {
    pub(crate) id: CommandActionId,
    pub(crate) label: &'static str,
    pub(crate) description: &'static str,
    pub(crate) keywords: &'static [&'static str],
}

impl CommandAction {
    pub(crate) fn matches(&self, query: &str) -> bool {
        if query.trim().is_empty() {
            return true;
        }
        let needle = query.trim().to_ascii_lowercase();
        let haystack = [self.label, self.description, &self.keywords.join(" ")]
            .join(" ")
            .to_ascii_lowercase();
        haystack.contains(&needle)
    }
}

pub(crate) const COMMAND_ACTIONS: &[CommandAction] = &[
    CommandAction {
        id: CommandActionId::OpenCapture,
        label: "Add",
        description: "Add a task with inline tokens",
        keywords: &["add", "capture", "task"],
    },
    CommandAction {
        id: CommandActionId::PromoteNext,
        label: "Promote to Next",
        description: "Mark selected tasks as next actions",
        keywords: &["next", "promote", "status"],
    },
    CommandAction {
        id: CommandActionId::MarkDone,
        label: "Mark done",
        description: "Complete selected tasks",
        keywords: &["done", "complete", "finish"],
    },
    CommandAction {
        id: CommandActionId::MoveToInbox,
        label: "Move to Inbox",
        description: "Send selected tasks back to inbox",
        keywords: &["inbox", "reset"],
    },
    CommandAction {
        id: CommandActionId::DeferTomorrow,
        label: "Defer until tomorrow",
        description: "Snooze selected task for 1 day",
        keywords: &["defer", "tomorrow", "schedule"],
    },
    CommandAction {
        id: CommandActionId::DeferNextWeek,
        label: "Defer until next week",
        description: "Snooze selected task for 7 days",
        keywords: &["defer", "week", "schedule"],
    },
    CommandAction {
        id: CommandActionId::Refresh,
        label: "Refresh now",
        description: "Reload active view",
        keywords: &["refresh", "reload"],
    },
];

#[derive(Clone, Copy)]
pub(crate) struct SampleSeed {
    pub(crate) text: &'static str,
    pub(crate) notes: Option<&'static str>,
    pub(crate) status: Option<TaskStatus>,
}

pub(crate) const SAMPLE_SEEDS: &[SampleSeed] = &[
    SampleSeed {
        text: "Review weekly metrics +Ops due:tomorrow @desk #metrics",
        notes: Some("Check dashboards before Monday stand-up"),
        status: Some(TaskStatus::Next),
    },
    SampleSeed {
        text: "Follow up with Acme procurement wait:Alex due:+3d @email",
        notes: Some("Waiting for revised contract terms"),
        status: Some(TaskStatus::Waiting),
    },
    SampleSeed {
        text: "Prep quarterly planning deck +Planning due:+1w @focus",
        notes: Some("Outline milestones and blockers"),
        status: Some(TaskStatus::Scheduled),
    },
    SampleSeed {
        text: "Capture retro ideas for team @brainstorm #retro",
        notes: Some("Bring to Friday's weekly review"),
        status: Some(TaskStatus::Inbox),
    },
    SampleSeed {
        text: "Audit backlog for parser refactor +Core #tech-debt",
        notes: Some("Identify candidates for next sprint"),
        status: Some(TaskStatus::Next),
    },
];

#[derive(Debug, Clone)]
pub(crate) struct CommandPaletteState {
    pub(crate) open: bool,
    pub(crate) query: String,
    pub(crate) selected: usize,
}

impl CommandPaletteState {
    pub(crate) fn new() -> Self {
        Self {
            open: false,
            query: String::new(),
            selected: 0,
        }
    }

    pub(crate) fn toggle(&mut self) {
        self.open = !self.open;
        if self.open {
            self.query.clear();
            self.selected = 0;
        }
    }

    pub(crate) fn close(&mut self) {
        self.open = false;
        self.query.clear();
        self.selected = 0;
    }

    pub(crate) fn filtered(&self) -> Vec<&'static CommandAction> {
        COMMAND_ACTIONS
            .iter()
            .filter(|action| action.matches(&self.query))
            .collect()
    }

    pub(crate) fn clamp_selection(&mut self) {
        let len = self.filtered().len().saturating_sub(1);
        if self.selected > len {
            self.selected = len;
        }
    }

    pub(crate) fn move_selection(&mut self, delta: i32) {
        let list = self.filtered();
        if list.is_empty() {
            self.selected = 0;
            return;
        }
        let len = list.len();
        let current = self.selected as i32;
        let mut next = current + delta;
        if next < 0 {
            next = (len as i32 - 1).max(0);
        } else if next >= len as i32 {
            next = 0;
        }
        self.selected = next as usize;
    }

    pub(crate) fn selected_action(&self) -> Option<&'static CommandAction> {
        self.filtered().get(self.selected).copied()
    }
}

#[derive(Debug, Clone)]
pub(crate) enum MutationKind {
    Promote(Vec<String>),
    Complete(Vec<String>),
    Inbox(Vec<String>),
    Defer { id: String, until: DateTime<Utc> },
    Rename { id: String, title: String },
    ChangeProject { id: String, project: Option<String> },
    ChangeContexts { id: String, contexts: Vec<String> },
    ChangeTags { id: String, tags: Vec<String> },
    ChangePriority { id: String, priority: u8 },
}

impl MutationKind {
    pub(crate) fn label(&self) -> &'static str {
        match self {
            MutationKind::Promote(_) => "promote",
            MutationKind::Complete(_) => "complete",
            MutationKind::Inbox(_) => "move to inbox",
            MutationKind::Defer { .. } => "defer",
            MutationKind::Rename { .. } => "rename",
            MutationKind::ChangeProject { .. } => "update project",
            MutationKind::ChangeContexts { .. } => "update contexts",
            MutationKind::ChangeTags { .. } => "update tags",
            MutationKind::ChangePriority { .. } => "update priority",
        }
    }
}
