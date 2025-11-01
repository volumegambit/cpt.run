use chrono::{DateTime, Utc};

use cpt_core::model::EnergyLevel;
use cpt_core::ViewSnapshot;

use crate::app::helpers::format_datetime;
use crate::app::state::ViewTab;

#[derive(Debug, Clone, Copy)]
pub(crate) enum ColumnAlignment {
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub(crate) struct TableColumn {
    pub label: &'static str,
    pub portion: u16,
    pub alignment: ColumnAlignment,
}

impl TableColumn {
    pub const fn left(label: &'static str, portion: u16) -> Self {
        Self {
            label,
            portion,
            alignment: ColumnAlignment::Left,
        }
    }

    pub const fn right(label: &'static str, portion: u16) -> Self {
        Self {
            label,
            portion,
            alignment: ColumnAlignment::Right,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TaskRow {
    pub id: String,
    pub cells: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct TaskTable {
    pub columns: Vec<TableColumn>,
    pub rows: Vec<TaskRow>,
}

#[derive(Debug, Clone)]
pub(crate) struct ProjectRow {
    pub cells: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ProjectTable {
    pub columns: Vec<TableColumn>,
    pub rows: Vec<ProjectRow>,
}

pub(crate) fn build_task_table(view: ViewTab, snapshot: &ViewSnapshot) -> TaskTable {
    let rows = snapshot
        .tasks
        .iter()
        .map(|task| TaskRow {
            id: task.id.clone(),
            cells: match view {
                ViewTab::All => vec![
                    task.title.clone(),
                    task.status.to_string(),
                    display_option(task.project.clone()),
                    display_list(&task.contexts),
                    display_list(&task.tags),
                    format_date(task.due_at),
                    format_priority(task.priority),
                    format_energy(task.energy),
                ],
                ViewTab::Inbox => vec![
                    task.title.clone(),
                    display_option(task.project.clone()),
                    display_list(&task.contexts),
                    display_list(&task.tags),
                    format_date(task.due_at),
                    format_priority(task.priority),
                ],
                ViewTab::Next => vec![
                    task.title.clone(),
                    display_option(task.project.clone()),
                    display_list(&task.contexts),
                    display_list(&task.tags),
                    format_energy(task.energy),
                    format_date(task.due_at),
                ],
                ViewTab::Waiting => vec![
                    task.title.clone(),
                    display_option(task.project.clone()),
                    display_list(&task.contexts),
                    display_list(&task.tags),
                    format_date(task.due_at),
                ],
                ViewTab::Scheduled => vec![
                    task.title.clone(),
                    display_option(task.project.clone()),
                    format_date(task.defer_until),
                    format_date(task.due_at),
                    display_list(&task.tags),
                ],
                ViewTab::Projects => vec![],
            },
        })
        .collect();

    let columns = match view {
        ViewTab::All => vec![
            TableColumn::left("Title", 6),
            TableColumn::left("Status", 2),
            TableColumn::left("Project", 3),
            TableColumn::left("Contexts", 3),
            TableColumn::left("Tags", 3),
            TableColumn::left("Due", 2),
            TableColumn::left("Priority", 1),
            TableColumn::left("Energy", 1),
        ],
        ViewTab::Inbox => vec![
            TableColumn::left("Title", 8),
            TableColumn::left("Project", 3),
            TableColumn::left("Contexts", 3),
            TableColumn::left("Tags", 3),
            TableColumn::left("Due", 2),
            TableColumn::left("Priority", 1),
        ],
        ViewTab::Next => vec![
            TableColumn::left("Title", 8),
            TableColumn::left("Project", 3),
            TableColumn::left("Contexts", 3),
            TableColumn::left("Tags", 3),
            TableColumn::left("Energy", 1),
            TableColumn::left("Due", 2),
        ],
        ViewTab::Waiting => vec![
            TableColumn::left("Title", 8),
            TableColumn::left("Project", 3),
            TableColumn::left("Waiting On", 4),
            TableColumn::left("Tags", 3),
            TableColumn::left("Due", 2),
        ],
        ViewTab::Scheduled => vec![
            TableColumn::left("Title", 8),
            TableColumn::left("Project", 3),
            TableColumn::left("Defer", 2),
            TableColumn::left("Due", 2),
            TableColumn::left("Tags", 3),
        ],
        ViewTab::Projects => Vec::new(),
    };

    TaskTable { columns, rows }
}

pub(crate) fn build_project_table(snapshot: &ViewSnapshot) -> ProjectTable {
    let rows = snapshot
        .projects
        .iter()
        .map(|summary| ProjectRow {
            cells: vec![
                summary.project.clone(),
                summary.total.to_string(),
                summary.next_actions.to_string(),
                summary.waiting.to_string(),
                summary.someday.to_string(),
            ],
        })
        .collect();

    let columns = vec![
        TableColumn::left("Project", 6),
        TableColumn::right("Total", 1),
        TableColumn::right("Next", 1),
        TableColumn::right("Waiting", 1),
        TableColumn::right("Someday", 1),
    ];

    ProjectTable { columns, rows }
}

fn display_option(value: Option<String>) -> String {
    value
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "—".into())
}

fn display_list(values: &[String]) -> String {
    if values.is_empty() {
        "—".into()
    } else {
        values.join(", ")
    }
}

fn format_date(date: Option<DateTime<Utc>>) -> String {
    date.map(format_datetime).unwrap_or_else(|| "—".into())
}

fn format_priority(priority: u8) -> String {
    if priority > 0 {
        format!("P{}", priority)
    } else {
        "—".into()
    }
}

fn format_energy(energy: Option<EnergyLevel>) -> String {
    energy
        .map(|value| value.as_str().to_string())
        .unwrap_or_else(|| "—".into())
}
