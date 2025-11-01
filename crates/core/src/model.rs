use std::fmt;
use std::str::FromStr;

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use clap::ValueEnum;
use serde::Serialize;
use ulid::Ulid;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Inbox,
    Next,
    Waiting,
    Scheduled,
    Someday,
    Done,
    Canceled,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Inbox => "inbox",
            TaskStatus::Next => "next",
            TaskStatus::Waiting => "waiting",
            TaskStatus::Scheduled => "scheduled",
            TaskStatus::Someday => "someday",
            TaskStatus::Done => "done",
            TaskStatus::Canceled => "canceled",
        }
    }

    pub fn default_for_waiting(waiting_fields_present: bool) -> Self {
        if waiting_fields_present {
            TaskStatus::Waiting
        } else {
            TaskStatus::Inbox
        }
    }
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for TaskStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "inbox" => Ok(TaskStatus::Inbox),
            "next" => Ok(TaskStatus::Next),
            "waiting" => Ok(TaskStatus::Waiting),
            "scheduled" => Ok(TaskStatus::Scheduled),
            "someday" => Ok(TaskStatus::Someday),
            "done" => Ok(TaskStatus::Done),
            "canceled" | "cancelled" => Ok(TaskStatus::Canceled),
            other => Err(anyhow!(
                "Unknown status '{}': expected inbox|next|waiting|scheduled|someday|done|canceled",
                other
            )),
        }
    }
}

impl ValueEnum for TaskStatus {
    fn value_variants<'a>() -> &'a [Self] {
        const VARIANTS: [TaskStatus; 7] = [
            TaskStatus::Inbox,
            TaskStatus::Next,
            TaskStatus::Waiting,
            TaskStatus::Scheduled,
            TaskStatus::Someday,
            TaskStatus::Done,
            TaskStatus::Canceled,
        ];
        &VARIANTS
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(clap::builder::PossibleValue::new(self.as_str()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EnergyLevel {
    Low,
    Med,
    High,
}

impl EnergyLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            EnergyLevel::Low => "low",
            EnergyLevel::Med => "med",
            EnergyLevel::High => "high",
        }
    }
}

impl fmt::Display for EnergyLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for EnergyLevel {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "low" => Ok(EnergyLevel::Low),
            "med" | "medium" => Ok(EnergyLevel::Med),
            "high" => Ok(EnergyLevel::High),
            other => Err(anyhow!(
                "Unknown energy level '{}': expected low|med|high",
                other
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortField {
    Due,
    Priority,
    Created,
}

impl FromStr for SortField {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "due" => Ok(SortField::Due),
            "priority" => Ok(SortField::Priority),
            "created" | "created_at" | "created-at" => Ok(SortField::Created),
            other => Err(anyhow!(
                "Unknown sort field '{}': expected due|priority|created",
                other
            )),
        }
    }
}

#[derive(Debug, Clone, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum ListView {
    Inbox,
    Next,
    Waiting,
    Scheduled,
    Someday,
    Projects,
    Done,
}

impl ListView {
    pub fn to_status(&self) -> Option<TaskStatus> {
        match self {
            ListView::Projects => None,
            ListView::Inbox => Some(TaskStatus::Inbox),
            ListView::Next => Some(TaskStatus::Next),
            ListView::Waiting => Some(TaskStatus::Waiting),
            ListView::Scheduled => Some(TaskStatus::Scheduled),
            ListView::Someday => Some(TaskStatus::Someday),
            ListView::Done => Some(TaskStatus::Done),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub notes: Option<String>,
    pub status: TaskStatus,
    pub project: Option<String>,
    pub areas: Vec<String>,
    pub contexts: Vec<String>,
    pub tags: Vec<String>,
    pub priority: u8,
    pub energy: Option<EnergyLevel>,
    pub time_estimate: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defer_until: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repeat: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waiting_on: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waiting_since: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct NewTask {
    pub title: String,
    pub notes: Option<String>,
    pub status: TaskStatus,
    pub project: Option<String>,
    pub areas: Vec<String>,
    pub contexts: Vec<String>,
    pub tags: Vec<String>,
    pub priority: u8,
    pub energy: Option<EnergyLevel>,
    pub time_estimate: Option<u32>,
    pub due_at: Option<DateTime<Utc>>,
    pub defer_until: Option<DateTime<Utc>>,
    pub repeat: Option<String>,
    pub waiting_on: Option<String>,
    pub waiting_since: Option<DateTime<Utc>>,
}

impl NewTask {
    pub fn into_insertable(self) -> InsertableTask {
        InsertableTask {
            id: Ulid::new().to_string(),
            data: self,
        }
    }
}

impl From<&Task> for NewTask {
    fn from(task: &Task) -> Self {
        Self {
            title: task.title.clone(),
            notes: task.notes.clone(),
            status: task.status,
            project: task.project.clone(),
            areas: task.areas.clone(),
            contexts: task.contexts.clone(),
            tags: task.tags.clone(),
            priority: task.priority,
            energy: task.energy,
            time_estimate: task.time_estimate,
            due_at: task.due_at,
            defer_until: task.defer_until,
            repeat: task.repeat.clone(),
            waiting_on: task.waiting_on.clone(),
            waiting_since: task.waiting_since,
        }
    }
}

pub struct InsertableTask {
    pub id: String,
    pub data: NewTask,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddOutcome {
    pub id: String,
    pub status: TaskStatus,
    pub title: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatusUpdate {
    pub id: String,
    pub changed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeleteResult {
    pub id: String,
    pub deleted: bool,
}

#[derive(Debug, Clone)]
pub struct ListFilters {
    pub view: Option<ListView>,
    pub status: Option<TaskStatus>,
    pub project: Option<String>,
    pub contexts: Vec<String>,
    pub tags: Vec<String>,
    pub due_before: Option<DateTime<Utc>>,
    pub defer_after: Option<DateTime<Utc>>,
    pub time_max: Option<u32>,
    pub energy: Option<EnergyLevel>,
    pub priority_min: Option<u8>,
    pub include_done: bool,
    pub sort: SortField,
    pub reverse: bool,
}

impl ListFilters {
    pub fn for_view(view: Option<ListView>) -> Self {
        let mut status = view.as_ref().and_then(|v| v.to_status());
        if matches!(view, Some(ListView::Scheduled)) {
            status = None;
        }
        let include_done = matches!(view, Some(ListView::Done));
        let sort = match view {
            Some(ListView::Next) | Some(ListView::Scheduled) => SortField::Due,
            Some(ListView::Someday) => SortField::Priority,
            Some(ListView::Inbox) | Some(ListView::Waiting) => SortField::Created,
            Some(ListView::Projects) | Some(ListView::Done) => SortField::Created,
            None => SortField::Due,
        };

        Self {
            view,
            status,
            project: None,
            contexts: Vec::new(),
            tags: Vec::new(),
            due_before: None,
            defer_after: None,
            time_max: None,
            energy: None,
            priority_min: None,
            include_done,
            sort,
            reverse: false,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectSummary {
    pub project: String,
    pub total: usize,
    pub next_actions: usize,
    pub waiting: usize,
    pub someday: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum ListOutputItem {
    Task(Box<Task>),
    Project(ProjectSummary),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_filters_for_view_sets_status_and_sort() {
        let next = ListFilters::for_view(Some(ListView::Next));
        assert_eq!(next.status, Some(TaskStatus::Next));
        assert_eq!(next.sort, SortField::Due);

        let waiting = ListFilters::for_view(Some(ListView::Waiting));
        assert_eq!(waiting.status, Some(TaskStatus::Waiting));
        assert_eq!(waiting.sort, SortField::Created);

        let projects = ListFilters::for_view(Some(ListView::Projects));
        assert!(projects.status.is_none());
        assert_eq!(projects.include_done, false);
    }

    #[test]
    fn default_for_waiting_matches_presence() {
        assert_eq!(TaskStatus::default_for_waiting(true), TaskStatus::Waiting);
        assert_eq!(TaskStatus::default_for_waiting(false), TaskStatus::Inbox);
    }
}
