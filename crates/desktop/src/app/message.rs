//! Message definitions passed around the desktop update loop.

use std::result::Result;

use cpt_core::model::AddOutcome;
use cpt_core::ViewSnapshot;
use iced::keyboard::Event as KeyboardEvent;
use iced::Task;

use crate::app::state::{CommandActionId, MutationKind, ViewTab};

#[derive(Debug, Clone)]
pub(crate) enum Message {
    ViewRequested(ViewTab),
    ViewLoaded(ViewTab, Result<ViewSnapshot, String>),
    RefreshTick,
    ToggleTheme,
    CaptureToggled,
    CaptureTextChanged(String),
    CaptureSubmit,
    CaptureCompleted(Result<AddOutcome, String>),
    CommandPaletteToggled,
    CommandPaletteClosed,
    CommandPaletteQueryChanged(String),
    CommandPaletteExecute(CommandActionId),
    MutationFinished(MutationKind, Result<(), String>),
    RowSelected(String),
    TaskTitlePressed(String),
    TaskProjectPressed(String),
    TaskContextsPressed(String),
    TaskTagsPressed(String),
    TaskPriorityPressed(String),
    InlineEditChanged(String),
    InlineEditSubmitted,
    InlineEditOptionSelected(String),
    Keyboard(KeyboardEvent),
}

pub(crate) type Effect = Task<Message>;
