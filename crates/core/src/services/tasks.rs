use std::collections::HashSet;

use anyhow::Result;
use chrono::{DateTime, Utc};

use crate::capture::CaptureInput;
use crate::config::AppConfig;
use crate::database::Database;
use crate::model::{
    AddOutcome, ListFilters, ListOutputItem, ListView, ProjectSummary, StatusUpdate, Task,
    TaskStatus,
};

#[derive(Debug, Clone)]
pub struct ViewSnapshot {
    pub filters: ListFilters,
    pub tasks: Vec<Task>,
    pub projects: Vec<ProjectSummary>,
}

impl ViewSnapshot {
    pub fn view(&self) -> Option<ListView> {
        self.filters.view.clone()
    }

    pub fn is_project_view(&self) -> bool {
        matches!(self.filters.view, Some(ListView::Projects))
    }
}

#[derive(Debug, Clone)]
pub struct TasksService {
    config: AppConfig,
}

impl TasksService {
    pub fn new(config: AppConfig) -> Result<Self> {
        Database::initialize(&config)?;
        Ok(Self { config })
    }

    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    pub fn list(&self, filters: &ListFilters) -> Result<ViewSnapshot> {
        let db = self.open_database()?;
        let mut tasks = Vec::new();
        let mut projects = Vec::new();

        for item in db.fetch_tasks(filters)? {
            match item {
                ListOutputItem::Task(task) => tasks.push(*task),
                ListOutputItem::Project(project) => projects.push(project),
            }
        }

        Ok(ViewSnapshot {
            filters: filters.clone(),
            tasks,
            projects,
        })
    }

    pub fn capture(&self, input: CaptureInput) -> Result<AddOutcome> {
        input.require_text()?;
        let mut db = self.open_database()?;
        db.handle_add(&input)
    }

    pub fn promote_to_next(&self, ids: &[String]) -> Result<Vec<StatusUpdate>> {
        let db = self.open_database()?;
        db.mark_next(ids)
    }

    pub fn mark_done(&self, ids: &[String]) -> Result<Vec<StatusUpdate>> {
        let db = self.open_database()?;
        db.mark_done(ids)
    }

    pub fn move_to_inbox(&self, ids: &[String]) -> Result<Vec<StatusUpdate>> {
        let db = self.open_database()?;
        db.mark_inbox(ids)
    }

    pub fn mark_someday(&self, ids: &[String]) -> Result<Vec<StatusUpdate>> {
        let db = self.open_database()?;
        db.mark_someday(ids)
    }

    pub fn defer_until(
        &self,
        id: &str,
        defer_until: Option<DateTime<Utc>>,
    ) -> Result<Option<Task>> {
        let db = self.open_database()?;
        let existing = db.fetch_task(id)?;
        let Some(task) = existing else {
            return Ok(None);
        };

        let mut updated = crate::model::NewTask::from(&task);
        updated.defer_until = defer_until;
        updated.status = match updated.status {
            TaskStatus::Inbox | TaskStatus::Next if defer_until.is_some() => TaskStatus::Scheduled,
            other => other,
        };
        db.update_task(id, &updated)
    }

    pub fn rename_task(&self, id: &str, title: &str) -> Result<Option<Task>> {
        let db = self.open_database()?;
        let existing = db.fetch_task(id)?;
        let Some(task) = existing else {
            return Ok(None);
        };

        let mut updated = crate::model::NewTask::from(&task);
        updated.title = title.to_string();
        db.update_task(id, &updated)
    }

    pub fn update_project(&self, id: &str, project: Option<String>) -> Result<Option<Task>> {
        let db = self.open_database()?;
        let existing = db.fetch_task(id)?;
        let Some(task) = existing else {
            return Ok(None);
        };

        let mut updated = crate::model::NewTask::from(&task);
        updated.project = project.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });
        db.update_task(id, &updated)
    }

    pub fn update_contexts(&self, id: &str, contexts: Vec<String>) -> Result<Option<Task>> {
        let db = self.open_database()?;
        let existing = db.fetch_task(id)?;
        let Some(task) = existing else {
            return Ok(None);
        };

        let mut updated = crate::model::NewTask::from(&task);
        updated.contexts = normalize_list(contexts);
        db.update_task(id, &updated)
    }

    pub fn update_tags(&self, id: &str, tags: Vec<String>) -> Result<Option<Task>> {
        let db = self.open_database()?;
        let existing = db.fetch_task(id)?;
        let Some(task) = existing else {
            return Ok(None);
        };

        let mut updated = crate::model::NewTask::from(&task);
        updated.tags = normalize_list(tags);
        db.update_task(id, &updated)
    }

    pub fn update_priority(&self, id: &str, priority: u8) -> Result<Option<Task>> {
        let db = self.open_database()?;
        let existing = db.fetch_task(id)?;
        let Some(task) = existing else {
            return Ok(None);
        };

        let mut updated = crate::model::NewTask::from(&task);
        updated.priority = priority.clamp(0, 3);
        db.update_task(id, &updated)
    }

    pub fn fetch_task(&self, id: &str) -> Result<Option<Task>> {
        let db = self.open_database()?;
        db.fetch_task(id)
    }

    fn open_database(&self) -> Result<Database> {
        Database::initialize(&self.config)
    }
}

fn normalize_list(tokens: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for token in tokens {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            continue;
        }
        let key = trimmed.to_ascii_lowercase();
        if seen.insert(key) {
            result.push(trimmed.to_string());
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TaskStatus;
    use chrono::Utc;
    use tempfile::TempDir;

    fn service_with_temp_dir() -> (TasksService, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = AppConfig::from_data_dir(temp_dir.path().to_path_buf()).unwrap();
        let service = TasksService::new(config).unwrap();
        (service, temp_dir)
    }

    fn capture_simple(service: &TasksService, title: &str) -> String {
        let mut input = CaptureInput::default();
        input.text = title.split_whitespace().map(|s| s.to_string()).collect();
        service.capture(input).unwrap().id
    }

    #[test]
    fn lists_tasks_per_view() {
        let (service, _guard) = service_with_temp_dir();
        let inbox_id = capture_simple(&service, "Process invoices");
        let mut next_input = CaptureInput::default();
        next_input.text = vec!["Follow".into(), "up".into()];
        next_input.status = Some(TaskStatus::Next);
        service.capture(next_input).unwrap();

        let inbox_filters = ListFilters::for_view(Some(ListView::Inbox));
        let next_filters = ListFilters::for_view(Some(ListView::Next));
        let inbox_snapshot = service.list(&inbox_filters).unwrap();
        let next_snapshot = service.list(&next_filters).unwrap();

        assert_eq!(inbox_snapshot.tasks.len(), 1);
        assert_eq!(inbox_snapshot.tasks[0].id, inbox_id);
        assert_eq!(next_snapshot.tasks.len(), 1);
    }

    #[test]
    fn promotes_and_completes_tasks() {
        let (service, _guard) = service_with_temp_dir();
        let id = capture_simple(&service, "Write unit tests");
        let updates = service.promote_to_next(&[id.clone()]).unwrap();
        assert!(updates.iter().any(|u| u.changed));

        let task = service.fetch_task(&id).unwrap().unwrap();
        assert_eq!(task.status, TaskStatus::Next);

        service.mark_done(&[id.clone()]).unwrap();
        let task = service.fetch_task(&id).unwrap().unwrap();
        assert_eq!(task.status, TaskStatus::Done);
    }

    #[test]
    fn defers_task_updates_status() {
        let (service, _guard) = service_with_temp_dir();
        let id = capture_simple(&service, "Review PRD");
        let defer_until = Utc::now() + chrono::Duration::days(2);
        let updated = service
            .defer_until(&id, Some(defer_until))
            .unwrap()
            .unwrap();
        assert_eq!(updated.status, TaskStatus::Scheduled);
        assert!(updated.defer_until.is_some());
    }
}
