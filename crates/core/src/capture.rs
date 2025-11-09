use std::fmt;

use crate::model::TaskStatus;

/// Normalized input for capturing a task from any client (CLI, TUI, desktop).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskInput {
    pub text: Vec<String>,
    pub notes: Option<String>,
    pub project: Option<String>,
    pub areas: Vec<String>,
    pub status: Option<TaskStatus>,
    pub contexts: Vec<String>,
    pub tags: Vec<String>,
    pub due_at: Option<String>,
    pub defer_until: Option<String>,
    pub time_estimate: Option<u32>,
    pub energy: Option<String>,
    pub priority: Option<u8>,
    pub waiting_on: Option<String>,
    pub waiting_since: Option<String>,
}

impl TaskInput {
    pub fn require_text(&self) -> Result<(), CaptureError> {
        if self.text.is_empty() {
            return Err(CaptureError::EmptyText);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureError {
    EmptyText,
}

impl fmt::Display for CaptureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CaptureError::EmptyText => write!(f, "Task text cannot be empty"),
        }
    }
}

impl std::error::Error for CaptureError {}
