use super::super::filters::{ActiveFilters, FilterColumn, FilterFacets, FilterOverlay};
use crate::model::{EnergyLevel, ListFilters, Task, TaskStatus};
use crate::tui::helpers::{
    centered_rect, compose_task_capture, format_task_detail_entries, join_prefixed, short_id,
};
use ratatui::layout::Rect;

#[test]
fn centered_rect_keeps_within_bounds() {
    let area = Rect {
        x: 0,
        y: 0,
        width: 80,
        height: 24,
    };
    let rect = centered_rect(40, 10, area);
    assert!(rect.x >= area.x);
    assert!(rect.y >= area.y);
    assert!(rect.width <= area.width);
    assert!(rect.height <= area.height);
    assert_eq!(rect.width, 40);
    assert_eq!(rect.height, 10);
}

#[test]
fn short_id_truncates_long_ids() {
    assert_eq!(short_id("abc"), "abc");
    assert_eq!(short_id("123456789"), "123456");
}

#[test]
fn join_prefixed_formats_values() {
    let values = vec!["home".to_string(), "work".to_string()];
    assert_eq!(join_prefixed(&values, "@"), "@home @work");
}

#[test]
fn active_filters_summary_formats_multiple_facets() {
    let mut filters = ActiveFilters::default();
    filters.project = Some("Acme".into());
    filters.contexts.insert("home".into());
    filters.tags.insert("ops".into());
    filters.priority_min = Some(2);

    assert_eq!(
        filters.summary().as_deref(),
        Some("project:Acme | ctx:@home | tag:#ops | priority≥2")
    );
}

#[test]
fn active_filters_apply_to_updates_list_filters() {
    let mut filters = ActiveFilters::default();
    filters.project = Some("Acme".into());
    filters.contexts.insert("home".into());
    filters.tags.insert("ops".into());
    filters.priority_min = Some(1);

    let mut list_filters = ListFilters::for_view(None);
    filters.apply_to(&mut list_filters);

    assert_eq!(list_filters.project.as_deref(), Some("Acme"));
    assert_eq!(list_filters.contexts, vec!["home".to_string()]);
    assert_eq!(list_filters.tags, vec!["ops".to_string()]);
    assert_eq!(list_filters.priority_min, Some(1));
}

#[test]
fn filter_overlay_clear_all_resets_state() {
    let tasks = vec![dummy_task("1", Some("Acme"), vec!["home"], vec!["ops"], 2)];
    let facets = FilterFacets::from_tasks(&tasks);
    let mut active = ActiveFilters::default();
    active.project = Some("Acme".into());
    active.contexts.insert("home".into());
    active.tags.insert("ops".into());
    active.priority_min = Some(2);

    let mut overlay = FilterOverlay::new(facets, &active);
    overlay.row_positions = [1, 1, 1, 1];

    overlay.clear_all();

    assert!(overlay.working.is_empty());
    assert_eq!(overlay.row_positions, [0, 0, 0, 0]);
}

#[test]
fn filter_overlay_toggle_project_selection_cycles() {
    let tasks = vec![dummy_task("1", Some("Acme"), vec!["home"], vec!["ops"], 1)];
    let facets = FilterFacets::from_tasks(&tasks);
    let active = ActiveFilters::default();
    let mut overlay = FilterOverlay::new(facets, &active);

    overlay.column = FilterColumn::Projects;
    overlay.row_positions[FilterColumn::Projects.index()] = 1;
    overlay.toggle_current();
    assert_eq!(overlay.working.project.as_deref(), Some("Acme"));

    overlay.toggle_current();
    assert!(overlay.working.project.is_none());
}

#[test]
fn compose_task_capture_emits_key_tokens() {
    let now = chrono::Utc::now();
    let task = Task {
        id: "task-1".into(),
        title: "Review PR".into(),
        notes: Some("Look at inline comments".into()),
        status: TaskStatus::Next,
        project: Some("Platform".into()),
        areas: vec!["eng".into()],
        contexts: vec!["office".into()],
        tags: vec!["infra".into()],
        priority: 2,
        energy: Some(EnergyLevel::Med),
        time_estimate: Some(45),
        due_at: Some(now),
        defer_until: None,
        repeat: None,
        created_at: now,
        updated_at: now,
        completed_at: None,
        waiting_on: None,
        waiting_since: None,
    };

    let capture = compose_task_capture(&task);
    assert!(capture.contains("Review PR"));
    assert!(capture.contains("+Platform"));
    assert!(capture.contains("@office"));
    assert!(capture.contains("#infra"));
    assert!(capture.contains("p:2"));
    assert!(capture.contains("t:45m"));
    assert!(capture.contains("e:med"));
}

#[test]
fn format_task_detail_entries_surfaces_metadata() {
    let now = chrono::Utc::now();
    let task = Task {
        id: "task-1".into(),
        title: "Review PR".into(),
        notes: Some("Line one\nLine two".into()),
        status: TaskStatus::Next,
        project: Some("Platform".into()),
        areas: vec!["eng".into()],
        contexts: vec!["office".into()],
        tags: vec!["infra".into()],
        priority: 2,
        energy: Some(EnergyLevel::Med),
        time_estimate: Some(45),
        due_at: Some(now),
        defer_until: None,
        repeat: None,
        created_at: now,
        updated_at: now,
        completed_at: None,
        waiting_on: None,
        waiting_since: None,
    };

    let entries = format_task_detail_entries(&task);
    assert!(entries
        .iter()
        .any(|(k, v)| k == "Title" && v == "Review PR"));
    assert!(entries.iter().any(|(k, v)| k == "Status" && v == "next"));
    assert!(entries
        .iter()
        .any(|(k, v)| k == "Project" && v == "Platform"));
    assert!(entries
        .iter()
        .any(|(k, v)| k == "Contexts" && v == "@office"));
    assert!(entries.iter().any(|(k, v)| k == "Tags" && v == "#infra"));
    assert!(entries.iter().any(|(k, v)| k == "Priority" && v == "2"));
    assert!(entries
        .iter()
        .any(|(k, v)| k == "Notes" && v.contains("Line two")));
}

fn dummy_task(
    id: &str,
    project: Option<&str>,
    contexts: Vec<&str>,
    tags: Vec<&str>,
    priority: u8,
) -> Task {
    let now = chrono::Utc::now();
    Task {
        id: id.to_string(),
        title: "Task".into(),
        notes: None,
        status: TaskStatus::Inbox,
        project: project.map(|p| p.to_string()),
        areas: Vec::new(),
        contexts: contexts.into_iter().map(|c| c.to_string()).collect(),
        tags: tags.into_iter().map(|t| t.to_string()).collect(),
        priority,
        energy: None,
        time_estimate: None,
        due_at: None,
        defer_until: None,
        repeat: None,
        created_at: now,
        updated_at: now,
        completed_at: None,
        waiting_on: None,
        waiting_since: None,
    }
}
