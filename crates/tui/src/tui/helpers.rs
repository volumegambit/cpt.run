use std::cmp::min;

use chrono::{DateTime, Local, Utc};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::model::Task;

pub const BG_BASE: Color = Color::Rgb(14, 17, 23);
pub const BG_PANEL: Color = Color::Rgb(22, 26, 34);
pub const BG_ACCENT: Color = Color::Rgb(32, 37, 47);
pub const FG_ACCENT: Color = Color::Rgb(120, 161, 255);

pub fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let w = min(width, area.width);
    let h = min(height, area.height);
    Rect {
        x: area.x + (area.width.saturating_sub(w)) / 2,
        y: area.y + (area.height.saturating_sub(h)) / 2,
        width: w,
        height: h,
    }
}

pub fn inset_rect(area: Rect, padding: u16) -> Rect {
    if area.width == 0 || area.height == 0 {
        return area;
    }
    let px = padding.min(area.width / 2);
    let py = padding.min(area.height / 2);
    Rect {
        x: area.x + px,
        y: area.y + py,
        width: area.width.saturating_sub(px * 2),
        height: area.height.saturating_sub(py * 2),
    }
}

pub fn short_id(id: &str) -> String {
    if id.len() <= 6 {
        id.to_string()
    } else {
        id[..6].to_string()
    }
}

pub fn join_prefixed(values: &[String], prefix: &str) -> String {
    values
        .iter()
        .map(|v| format!("{}{}", prefix, v))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn compose_task_capture(task: &Task) -> String {
    let mut components = Vec::new();
    components.push(task.title.clone());

    if let Some(project) = &task.project {
        if !project.is_empty() {
            components.push(format!("+{}", project));
        }
    }

    for area in &task.areas {
        if !area.is_empty() {
            components.push(format!("&{}", area));
        }
    }

    for context in &task.contexts {
        if !context.is_empty() {
            components.push(format!("@{}", context));
        }
    }

    for tag in &task.tags {
        if !tag.is_empty() {
            components.push(format!("#{}", tag));
        }
    }

    if let Some(priority) = Some(task.priority).filter(|p| *p > 0) {
        components.push(format!("p:{}", priority.min(3)));
    }

    if let Some(time) = task.time_estimate {
        components.push(format!("t:{}m", time));
    }

    if let Some(energy) = task.energy {
        components.push(format!("e:{}", energy.as_str()));
    }

    if let Some(due_at) = task.due_at {
        components.push(format!("due:{}", due_at.format("%Y-%m-%d")));
    }

    if let Some(defer_until) = task.defer_until {
        components.push(format!("defer:{}", defer_until.format("%Y-%m-%d")));
    }

    if let Some(waiting_on) = &task.waiting_on {
        if !waiting_on.is_empty() {
            components.push(format!("wait:{}", waiting_on));
        }
    }

    if components.is_empty() {
        task.title.clone()
    } else {
        components.join(" ")
    }
}

pub fn format_task_detail_entries(task: &Task) -> Vec<(String, String)> {
    let mut entries = Vec::new();
    entries.push((String::from("Title"), task.title.clone()));
    entries.push((String::from("Status"), task.status.as_str().to_string()));
    entries.push((String::from("ID"), task.id.clone()));

    if let Some(project) = &task.project {
        if !project.is_empty() {
            entries.push((String::from("Project"), project.clone()));
        }
    }
    if !task.areas.is_empty() {
        entries.push((String::from("Areas"), task.areas.join(", ")));
    }
    if !task.contexts.is_empty() {
        let contexts: Vec<String> = task.contexts.iter().map(|c| format!("@{}", c)).collect();
        entries.push((String::from("Contexts"), contexts.join(" ")));
    }
    if !task.tags.is_empty() {
        let tags: Vec<String> = task.tags.iter().map(|t| format!("#{}", t)).collect();
        entries.push((String::from("Tags"), tags.join(" ")));
    }
    if task.priority > 0 {
        entries.push((String::from("Priority"), task.priority.to_string()));
    }
    if let Some(energy) = task.energy {
        entries.push((String::from("Energy"), energy.as_str().to_string()));
    }
    if let Some(minutes) = task.time_estimate {
        entries.push((String::from("Estimate"), format!("{} min", minutes)));
    }
    let due = format_opt_datetime(task.due_at.as_ref());
    if !due.is_empty() {
        entries.push((String::from("Due"), due));
    }
    let defer = format_opt_datetime(task.defer_until.as_ref());
    if !defer.is_empty() {
        entries.push((String::from("Start"), defer));
    }
    if let Some(waiting_on) = &task.waiting_on {
        if !waiting_on.is_empty() {
            entries.push((String::from("Waiting on"), waiting_on.clone()));
        }
    }
    let waiting_since = format_opt_datetime(task.waiting_since.as_ref());
    if !waiting_since.is_empty() {
        entries.push((String::from("Waiting since"), waiting_since));
    }
    entries.push((
        String::from("Created"),
        format_opt_datetime(Some(&task.created_at)),
    ));
    entries.push((
        String::from("Updated"),
        format_opt_datetime(Some(&task.updated_at)),
    ));

    if let Some(notes) = &task.notes {
        if !notes.trim().is_empty() {
            entries.push((String::from("Notes"), notes.clone()));
        }
    }

    entries
}

pub fn build_help_lines() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Tab / Shift+Tab", "Switch GTD views"),
        ("j / k or ↓ / ↑", "Move selection"),
        ("q", "Quit"),
        ("Enter", "Toggle task detail overlay"),
        ("h", "Toggle this help overlay"),
        ("Shift+Enter", "Insert newline while adding or editing"),
        ("a", "Capture a new task"),
        ("e", "Edit selected task"),
        ("n", "Promote to Next actions"),
        ("s", "Move to Someday/Maybe"),
        ("i", "Send back to Inbox"),
        ("d", "Mark as Done"),
        ("x / Delete", "Delete task (with confirmation)"),
        ("f", "Open filter picker"),
        ("/", "Command palette"),
        ("C (in filter)", "Clear all filters"),
        ("r", "Refresh from storage"),
        ("Esc", "Cancel/close overlays"),
    ]
}

pub fn accent_title(text: &str) -> Line<'static> {
    Line::from(vec![Span::styled(
        text.to_owned(),
        Style::default().fg(FG_ACCENT).add_modifier(Modifier::BOLD),
    )])
}

pub fn format_opt_datetime(value: Option<&DateTime<Utc>>) -> String {
    value
        .map(|dt| {
            let local: DateTime<Local> = (*dt).into();
            local.format("%Y-%m-%d %H:%M").to_string()
        })
        .unwrap_or_default()
}
