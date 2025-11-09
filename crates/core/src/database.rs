use std::collections::HashMap;

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{named_params, types::Value, Connection, Row, ToSql};

use crate::capture::TaskInput;
use crate::config::AppConfig;
use crate::model::{
    AddOutcome, DeleteResult, EnergyLevel, ListFilters, ListOutputItem, ListView, ProjectSummary,
    StatusUpdate, Task, TaskStatus,
};
use crate::parser;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn initialize(config: &AppConfig) -> Result<Self> {
        let conn = Connection::open(config.db_path()).with_context(|| {
            format!("Failed to open database at {}", config.db_path().display())
        })?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .context("Failed to configure SQLite WAL mode")?;

        let db = Self { conn };
        db.apply_migrations()?;
        Ok(db)
    }

    pub fn handle_add(&mut self, input: &TaskInput) -> Result<AddOutcome> {
        let (insertable, outcome) = parser::prepare_new_task(input)?;
        self.insert_task(&insertable)?;
        Ok(outcome)
    }

    pub fn fetch_tasks(&self, filters: &ListFilters) -> Result<Vec<ListOutputItem>> {
        if matches!(filters.view, Some(ListView::Projects)) {
            return self.fetch_projects(filters);
        }

        let mut sql = String::from(
            "SELECT id, title, notes, status, project, areas, contexts, tags, priority, energy, \
            time_estimate, due_at, defer_until, repeat, created_at, updated_at, completed_at, waiting_on, waiting_since \
            FROM tasks WHERE 1=1",
        );
        let mut values: Vec<Value> = Vec::new();

        if let Some(status) = filters.status {
            sql.push_str(" AND status = ?");
            values.push(Value::from(status.as_str().to_string()));
        } else if !filters.include_done {
            sql.push_str(" AND status NOT IN ('done','canceled')");
        }

        if matches!(filters.view, Some(ListView::Scheduled)) {
            sql.push_str(
                " AND ((status = 'scheduled') OR ((status IN ('inbox','next')) AND (due_at IS NOT NULL OR defer_until IS NOT NULL)))",
            );
        }

        if let Some(project) = &filters.project {
            sql.push_str(" AND project = ?");
            values.push(Value::from(project.clone()));
        }

        for ctx in &filters.contexts {
            let needle = format!("\"{}\"", ctx);
            sql.push_str(" AND instr(contexts, ?) > 0");
            values.push(Value::from(needle));
        }

        for tag in &filters.tags {
            let needle = format!("\"{}\"", tag);
            sql.push_str(" AND instr(tags, ?) > 0");
            values.push(Value::from(needle));
        }

        if let Some(due_before) = filters.due_before {
            sql.push_str(" AND due_at IS NOT NULL AND due_at <= ?");
            values.push(Value::from(due_before.to_rfc3339()));
        }

        if let Some(defer_after) = filters.defer_after {
            sql.push_str(" AND defer_until IS NOT NULL AND defer_until >= ?");
            values.push(Value::from(defer_after.to_rfc3339()));
        }

        if let Some(limit) = filters.time_max {
            sql.push_str(" AND (time_estimate IS NULL OR time_estimate <= ?)");
            values.push(Value::from(limit as i64));
        }

        if let Some(energy) = filters.energy {
            sql.push_str(" AND (energy = ?)");
            values.push(Value::from(energy.as_str().to_string()));
        }

        if let Some(priority) = filters.priority_min {
            sql.push_str(" AND priority >= ?");
            values.push(Value::from(priority as i64));
        }

        sql.push_str(&build_order_clause(filters));

        let mut stmt = self.conn.prepare(&sql)?;
        let param_refs: Vec<&dyn ToSql> = values.iter().map(|v| v as &dyn ToSql).collect();
        let mut rows = stmt.query(&param_refs[..])?;
        let mut tasks = Vec::new();
        while let Some(row) = rows.next()? {
            tasks.push(ListOutputItem::Task(Box::new(self.map_task(row)?)));
        }
        Ok(tasks)
    }

    pub fn mark_done(&self, ids: &[String]) -> Result<Vec<StatusUpdate>> {
        let now = Utc::now().to_rfc3339();
        self.update_status(ids, TaskStatus::Done, Some(now))
    }

    pub fn mark_next(&self, ids: &[String]) -> Result<Vec<StatusUpdate>> {
        self.update_status(ids, TaskStatus::Next, None)
    }

    pub fn mark_someday(&self, ids: &[String]) -> Result<Vec<StatusUpdate>> {
        self.update_status(ids, TaskStatus::Someday, None)
    }

    pub fn mark_inbox(&self, ids: &[String]) -> Result<Vec<StatusUpdate>> {
        self.update_status(ids, TaskStatus::Inbox, None)
    }

    fn update_status(
        &self,
        ids: &[String],
        status: TaskStatus,
        completed_at: Option<String>,
    ) -> Result<Vec<StatusUpdate>> {
        let mut results = Vec::new();
        let updated_ts = Utc::now().to_rfc3339();
        let completed_ref = completed_at.as_deref();
        for id in ids {
            let updated = self.conn.execute(
                "UPDATE tasks SET status = :status, completed_at = :completed, updated_at = :updated WHERE id = :id",
                named_params![
                    ":status": status.as_str(),
                    ":completed": completed_ref,
                    ":updated": updated_ts,
                    ":id": id,
                ],
            )?;
            results.push(StatusUpdate {
                id: id.to_string(),
                changed: updated > 0,
            });
        }
        Ok(results)
    }

    pub fn delete_tasks(&self, ids: &[String]) -> Result<Vec<DeleteResult>> {
        let mut results = Vec::new();
        for id in ids {
            let affected = self
                .conn
                .execute("DELETE FROM tasks WHERE id = :id", named_params![":id": id])?;
            results.push(DeleteResult {
                id: id.to_string(),
                deleted: affected > 0,
            });
        }
        Ok(results)
    }

    pub fn fetch_task(&self, id: &str) -> Result<Option<Task>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, notes, status, project, areas, contexts, tags, priority, energy, \
             time_estimate, due_at, defer_until, repeat, created_at, updated_at, completed_at, \
             waiting_on, waiting_since \
             FROM tasks WHERE id = ? LIMIT 1",
        )?;
        let mut rows = stmt.query([id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(self.map_task(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn update_task(&self, id: &str, updated: &crate::model::NewTask) -> Result<Option<Task>> {
        let existing = match self.fetch_task(id)? {
            Some(task) => task,
            None => return Ok(None),
        };

        let areas_json = serde_json::to_string(&if updated.areas.is_empty() {
            existing.areas.clone()
        } else {
            updated.areas.clone()
        })?;
        let contexts_json = serde_json::to_string(&updated.contexts)?;
        let tags_json = serde_json::to_string(&updated.tags)?;

        let energy = updated.energy.as_ref().map(|e| e.as_str().to_string());
        let time_estimate = updated.time_estimate.map(|v| v as i64);
        let due_at = updated.due_at.map(|dt| dt.to_rfc3339());
        let defer_until = updated.defer_until.map(|dt| dt.to_rfc3339());
        let waiting_since = updated.waiting_since.map(|dt| dt.to_rfc3339());
        let waiting_on = updated.waiting_on.clone();
        let notes = updated.notes.clone().or(existing.notes.clone());
        let repeat = updated.repeat.clone().or(existing.repeat.clone());

        let now = Utc::now().to_rfc3339();
        let completed_at = if updated.status == TaskStatus::Done {
            existing
                .completed_at
                .map(|dt| dt.to_rfc3339())
                .or_else(|| Some(now.clone()))
        } else {
            None
        };

        self.conn.execute(
            "UPDATE tasks SET
                title = :title,
                notes = :notes,
                status = :status,
                project = :project,
                areas = :areas,
                contexts = :contexts,
                tags = :tags,
                priority = :priority,
                energy = :energy,
                time_estimate = :time_estimate,
                due_at = :due_at,
                defer_until = :defer_until,
                repeat = :repeat,
                updated_at = :updated_at,
                completed_at = :completed_at,
                waiting_on = :waiting_on,
                waiting_since = :waiting_since
             WHERE id = :id",
            named_params![
                ":title": &updated.title,
                ":notes": notes,
                ":status": updated.status.as_str(),
                ":project": updated.project.as_deref(),
                ":areas": areas_json,
                ":contexts": contexts_json,
                ":tags": tags_json,
                ":priority": updated.priority as i64,
                ":energy": energy,
                ":time_estimate": time_estimate,
                ":due_at": due_at,
                ":defer_until": defer_until,
                ":repeat": repeat,
                ":updated_at": now,
                ":completed_at": completed_at,
                ":waiting_on": waiting_on,
                ":waiting_since": waiting_since,
                ":id": id,
            ],
        )?;

        self.fetch_task(id)
    }

    fn insert_task(&self, insertable: &crate::model::InsertableTask) -> Result<()> {
        let now = Utc::now();
        let data = &insertable.data;
        let areas_json = serde_json::to_string(&data.areas)?;
        let contexts_json = serde_json::to_string(&data.contexts)?;
        let tags_json = serde_json::to_string(&data.tags)?;
        let energy = data.energy.map(|e| e.as_str().to_string());
        let time_estimate = data.time_estimate.map(|v| v as i64);
        let due_at = data.due_at.map(|dt| dt.to_rfc3339());
        let defer_until = data.defer_until.map(|dt| dt.to_rfc3339());
        let repeat = data.repeat.clone();
        let waiting_on = data.waiting_on.clone();
        let waiting_since = data.waiting_since.map(|dt| dt.to_rfc3339());

        self.conn.execute(
            "INSERT INTO tasks (
                id, title, notes, status, project, areas, contexts, tags, priority, energy, time_estimate,
                due_at, defer_until, repeat, created_at, updated_at, completed_at, waiting_on, waiting_since
            ) VALUES (
                :id, :title, :notes, :status, :project, :areas, :contexts, :tags, :priority, :energy, :time_estimate,
                :due_at, :defer_until, :repeat, :created_at, :updated_at, NULL, :waiting_on, :waiting_since
            )",
            named_params![
                ":id": &insertable.id,
                ":title": &data.title,
                ":notes": data.notes.as_deref(),
                ":status": data.status.as_str(),
                ":project": data.project.as_deref(),
                ":areas": areas_json,
                ":contexts": contexts_json,
                ":tags": tags_json,
                ":priority": data.priority as i64,
                ":energy": energy,
                ":time_estimate": time_estimate,
                ":due_at": due_at,
                ":defer_until": defer_until,
                ":repeat": repeat,
                ":created_at": now.to_rfc3339(),
                ":updated_at": now.to_rfc3339(),
                ":waiting_on": waiting_on,
                ":waiting_since": waiting_since,
            ],
        )?;
        Ok(())
    }

    fn fetch_projects(&self, filters: &ListFilters) -> Result<Vec<ListOutputItem>> {
        let mut sql = String::from(
            "SELECT project, status, COUNT(*) AS total FROM tasks WHERE project IS NOT NULL AND project <> ''",
        );
        let mut values: Vec<Value> = Vec::new();

        if let Some(status) = filters.status {
            sql.push_str(" AND status = ?");
            values.push(Value::from(status.as_str().to_string()));
        }

        sql.push_str(" GROUP BY project, status");

        let mut stmt = self.conn.prepare(&sql)?;
        let param_refs: Vec<&dyn ToSql> = values.iter().map(|v| v as &dyn ToSql).collect();
        let mut rows = stmt.query(&param_refs[..])?;
        let mut aggregates: HashMap<String, ProjectSummary> = HashMap::new();

        while let Some(row) = rows.next()? {
            let project: String = row.get(0)?;
            let status: String = row.get(1)?;
            let count: i64 = row.get(2)?;
            let entry = aggregates
                .entry(project.clone())
                .or_insert_with(|| ProjectSummary {
                    project: project.clone(),
                    total: 0,
                    next_actions: 0,
                    waiting: 0,
                    someday: 0,
                });
            entry.total += count as usize;
            match status.as_str() {
                "next" => entry.next_actions += count as usize,
                "waiting" => entry.waiting += count as usize,
                "someday" => entry.someday += count as usize,
                _ => {}
            }
        }

        let mut summaries: Vec<ProjectSummary> = aggregates.into_values().collect();
        summaries.sort_by(|a, b| a.project.cmp(&b.project));
        Ok(summaries.into_iter().map(ListOutputItem::Project).collect())
    }

    fn map_task(&self, row: &Row<'_>) -> Result<Task> {
        let energy: Option<String> = row.get(9)?;
        let energy = match energy {
            Some(v) if !v.is_empty() => Some(v.parse::<EnergyLevel>()?),
            _ => None,
        };

        Ok(Task {
            id: row.get(0)?,
            title: row.get(1)?,
            notes: row.get(2)?,
            status: row.get::<_, String>(3)?.parse()?,
            project: row.get(4)?,
            areas: parse_string_list(row.get::<_, Option<String>>(5)?),
            contexts: parse_string_list(row.get::<_, Option<String>>(6)?),
            tags: parse_string_list(row.get::<_, Option<String>>(7)?),
            priority: row.get::<_, i64>(8)? as u8,
            energy,
            time_estimate: row.get::<_, Option<i64>>(10)?.map(|v| v as u32),
            due_at: parse_datetime(row.get::<_, Option<String>>(11)?),
            defer_until: parse_datetime(row.get::<_, Option<String>>(12)?),
            repeat: row.get(13)?,
            created_at: parse_datetime_required(row.get::<_, String>(14)?)?,
            updated_at: parse_datetime_required(row.get::<_, String>(15)?)?,
            completed_at: parse_datetime(row.get::<_, Option<String>>(16)?),
            waiting_on: row.get(17)?,
            waiting_since: parse_datetime(row.get::<_, Option<String>>(18)?),
        })
    }

    fn apply_migrations(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT);
             CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                notes TEXT,
                status TEXT NOT NULL,
                project TEXT,
                areas TEXT DEFAULT '[]',
                contexts TEXT DEFAULT '[]',
                tags TEXT DEFAULT '[]',
                priority INTEGER NOT NULL DEFAULT 0,
                energy TEXT,
                time_estimate INTEGER,
                due_at TEXT,
                defer_until TEXT,
                repeat TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                completed_at TEXT,
                waiting_on TEXT,
                waiting_since TEXT
             );
             CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
             CREATE INDEX IF NOT EXISTS idx_tasks_project ON tasks(project);
             CREATE INDEX IF NOT EXISTS idx_tasks_due ON tasks(due_at);
            ",
        )?;
        Ok(())
    }
}

fn parse_string_list(raw: Option<String>) -> Vec<String> {
    raw.and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
        .unwrap_or_default()
}

fn parse_datetime(raw: Option<String>) -> Option<DateTime<Utc>> {
    raw.and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
        .map(|dt| dt.with_timezone(&Utc))
}

fn parse_datetime_required(raw: String) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&raw)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| anyhow!("Failed to parse timestamp '{}': {}", raw, e))
}

fn build_order_clause(filters: &ListFilters) -> String {
    match filters.sort {
        crate::model::SortField::Due => {
            if filters.view.is_none() {
                if filters.reverse {
                    " ORDER BY \
                        CASE status \
                            WHEN 'next' THEN 0 \
                            WHEN 'scheduled' THEN 1 \
                            WHEN 'waiting' THEN 2 \
                            WHEN 'inbox' THEN 3 \
                            WHEN 'someday' THEN 4 \
                            ELSE 5 \
                        END DESC,
                        due_at IS NULL DESC,
                        due_at DESC,
                        priority ASC,
                        created_at DESC"
                        .into()
                } else {
                    " ORDER BY \
                        CASE status \
                            WHEN 'next' THEN 0 \
                            WHEN 'scheduled' THEN 1 \
                            WHEN 'waiting' THEN 2 \
                            WHEN 'inbox' THEN 3 \
                            WHEN 'someday' THEN 4 \
                            ELSE 5 \
                        END,
                        due_at IS NULL,
                        due_at ASC,
                        priority DESC,
                        created_at ASC"
                        .into()
                }
            } else if filters.reverse {
                " ORDER BY due_at IS NULL DESC, due_at DESC, priority ASC, time_estimate DESC, created_at DESC".into()
            } else {
                " ORDER BY due_at IS NULL, due_at ASC, priority DESC, time_estimate ASC, created_at ASC".into()
            }
        }
        crate::model::SortField::Priority => {
            if filters.reverse {
                " ORDER BY priority ASC, due_at DESC, created_at DESC".into()
            } else {
                " ORDER BY priority DESC, due_at ASC, created_at ASC".into()
            }
        }
        crate::model::SortField::Created => {
            if filters.reverse {
                " ORDER BY created_at DESC".into()
            } else {
                " ORDER BY created_at ASC".into()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::TaskInput;
    use crate::model::{ListView, TaskStatus};
    use tempfile::TempDir;

    fn temp_config() -> (AppConfig, TempDir) {
        let dir = TempDir::new().expect("temp dir");
        let data_dir = dir.path().to_path_buf();
        std::fs::create_dir_all(&data_dir).expect("create data dir");
        let config = AppConfig::from_data_dir(data_dir).expect("config");
        (config, dir)
    }

    #[test]
    fn build_order_respects_reverse() {
        let filters = ListFilters {
            view: None,
            status: None,
            project: None,
            contexts: vec![],
            tags: vec![],
            due_before: None,
            defer_after: None,
            time_max: None,
            energy: None,
            priority_min: None,
            include_done: false,
            sort: crate::model::SortField::Due,
            reverse: true,
        };
        let clause = build_order_clause(&filters);
        assert!(clause.contains("due_at DESC"));
    }

    #[test]
    fn all_view_orders_by_status_then_due() {
        let (config, _dir) = temp_config();
        let mut db = Database::initialize(&config).expect("init db");

        let add = |title: &str, status: TaskStatus, due: Option<&str>| {
            let mut text = vec![title.to_string()];
            if let Some(due_str) = due {
                text.push(format!("due:{}", due_str));
            }
            TaskInput {
                text,
                notes: None,
                project: None,
                areas: vec![],
                status: Some(status),
                contexts: vec![],
                tags: vec![],
                due_at: None,
                defer_until: None,
                time_estimate: None,
                energy: None,
                priority: None,
                waiting_on: None,
                waiting_since: None,
            }
        };

        let next = db
            .handle_add(&add("Next action", TaskStatus::Next, Some("2025-01-02")))
            .expect("add next");
        let scheduled = db
            .handle_add(&add("Scheduled", TaskStatus::Scheduled, Some("2025-01-01")))
            .expect("add scheduled");
        let waiting = db
            .handle_add(&add("Waiting", TaskStatus::Waiting, None))
            .expect("add waiting");
        let inbox = db
            .handle_add(&add("Inbox", TaskStatus::Inbox, None))
            .expect("add inbox");

        let filters = ListFilters::for_view(None);
        let items = db.fetch_tasks(&filters).expect("fetch all");
        let ids: Vec<String> = items
            .into_iter()
            .filter_map(|item| match item {
                ListOutputItem::Task(task) => Some(task.id),
                _ => None,
            })
            .collect();

        assert_eq!(ids.len(), 4);
        assert_eq!(ids[0], next.id);
        assert_eq!(ids[1], scheduled.id);
        assert_eq!(ids[2], waiting.id);
        assert_eq!(ids[3], inbox.id);
    }

    #[test]
    fn handle_add_and_fetch_roundtrip() {
        let (config, _dir) = temp_config();
        let mut db = Database::initialize(&config).expect("initialize db");

        let args = TaskInput {
            text: vec![
                "Write".into(),
                "release".into(),
                "+Website".into(),
                "@desk".into(),
                "due:2025-01-01".into(),
            ],
            notes: Some("Publish announcement".into()),
            project: None,
            areas: vec![],
            status: None,
            contexts: vec![],
            tags: vec![],
            due_at: None,
            defer_until: None,
            time_estimate: Some(45),
            energy: Some("med".into()),
            priority: Some(2),
            waiting_on: None,
            waiting_since: None,
        };

        let outcome = db.handle_add(&args).expect("add task");
        assert_eq!(outcome.status, TaskStatus::Inbox);

        let filters = ListFilters::for_view(None);
        let items = db.fetch_tasks(&filters).expect("fetch tasks");
        assert_eq!(items.len(), 1);
        match &items[0] {
            ListOutputItem::Task(task) => {
                let task = task.as_ref();
                assert_eq!(task.id, outcome.id);
                assert_eq!(task.project.as_deref(), Some("Website"));
                assert_eq!(task.contexts, vec!["desk".to_string()]);
                assert_eq!(task.priority, 2);
                assert_eq!(task.time_estimate, Some(45));
            }
            _ => panic!("expected task"),
        }
    }

    #[test]
    fn mark_done_and_delete_affect_storage() {
        let (config, _dir) = temp_config();
        let mut db = Database::initialize(&config).expect("initialize db");

        let args = TaskInput {
            text: vec!["Follow".into(), "Up".into()],
            notes: None,
            project: None,
            areas: vec![],
            status: None,
            contexts: vec![],
            tags: vec![],
            due_at: None,
            defer_until: None,
            time_estimate: None,
            energy: None,
            priority: None,
            waiting_on: None,
            waiting_since: None,
        };

        let outcome = db.handle_add(&args).expect("add task");
        let mark_results = db.mark_done(&[outcome.id.clone()]).expect("mark done");
        assert!(mark_results[0].changed);

        let done_filters = ListFilters::for_view(Some(ListView::Done));
        let done_items = db.fetch_tasks(&done_filters).expect("fetch done");
        assert_eq!(done_items.len(), 1);

        let delete = db.delete_tasks(&[outcome.id.clone()]).expect("delete task");
        assert!(delete[0].deleted);

        let remaining = db.fetch_tasks(&done_filters).expect("fetch after delete");
        assert!(remaining.is_empty());
    }

    #[test]
    fn mark_next_updates_status() {
        let (config, _dir) = temp_config();
        let mut db = Database::initialize(&config).expect("initialize db");

        let args = TaskInput {
            text: vec!["Outline".into(), "plan".into()],
            notes: None,
            project: None,
            areas: vec![],
            status: None,
            contexts: vec![],
            tags: vec![],
            due_at: None,
            defer_until: None,
            time_estimate: None,
            energy: None,
            priority: None,
            waiting_on: None,
            waiting_since: None,
        };

        let outcome = db.handle_add(&args).expect("add task");
        let updates = db.mark_next(&[outcome.id.clone()]).expect("mark next");
        assert!(updates[0].changed);

        let filters = ListFilters::for_view(Some(ListView::Next));
        let items = db.fetch_tasks(&filters).expect("fetch next");
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn mark_someday_and_inbox_cycle_status() {
        let (config, _dir) = temp_config();
        let mut db = Database::initialize(&config).expect("initialize db");

        let args = TaskInput {
            text: vec!["Outline".into(), "plan".into()],
            notes: None,
            project: None,
            areas: vec![],
            status: None,
            contexts: vec![],
            tags: vec![],
            due_at: None,
            defer_until: None,
            time_estimate: None,
            energy: None,
            priority: None,
            waiting_on: None,
            waiting_since: None,
        };

        let outcome = db.handle_add(&args).expect("add task");
        db.mark_next(&[outcome.id.clone()]).expect("mark next");
        db.mark_someday(&[outcome.id.clone()])
            .expect("mark someday");

        let someday_filters = ListFilters::for_view(Some(ListView::Someday));
        let someday_items = db.fetch_tasks(&someday_filters).expect("fetch someday");
        assert_eq!(someday_items.len(), 1);
        if let ListOutputItem::Task(task) = &someday_items[0] {
            let task = task.as_ref();
            assert_eq!(task.status, TaskStatus::Someday);
        } else {
            panic!("expected task");
        }

        db.mark_inbox(&[outcome.id.clone()]).expect("mark inbox");

        let filters = ListFilters::for_view(Some(ListView::Inbox));
        let items = db.fetch_tasks(&filters).expect("fetch inbox");
        assert_eq!(items.len(), 1);
        if let ListOutputItem::Task(task) = &items[0] {
            let task = task.as_ref();
            assert_eq!(task.status, TaskStatus::Inbox);
        } else {
            panic!("expected task");
        }
    }

    #[test]
    fn scheduled_view_contains_due_tasks() {
        let (config, _dir) = temp_config();
        let mut db = Database::initialize(&config).expect("initialize db");

        let args = TaskInput {
            text: vec![
                "Prepare".into(),
                "slides".into(),
                "due:2025-01-01".into(),
                "defer:2024-12-01".into(),
            ],
            notes: None,
            project: None,
            areas: vec![],
            status: None,
            contexts: vec![],
            tags: vec![],
            due_at: None,
            defer_until: None,
            time_estimate: None,
            energy: None,
            priority: None,
            waiting_on: None,
            waiting_since: None,
        };

        let outcome = db.handle_add(&args).expect("add task");
        let filters = ListFilters::for_view(Some(ListView::Scheduled));
        let items = db.fetch_tasks(&filters).expect("fetch scheduled");
        assert_eq!(items.len(), 1);
        match &items[0] {
            ListOutputItem::Task(task) => {
                let task = task.as_ref();
                assert_eq!(task.id, outcome.id);
                assert!(task.due_at.is_some());
            }
            _ => panic!("expected task"),
        }
    }

    #[test]
    fn scheduled_view_excludes_waiting_tasks() {
        let (config, _dir) = temp_config();
        let mut db = Database::initialize(&config).expect("initialize db");

        let args = TaskInput {
            text: vec![
                "Await".into(),
                "response".into(),
                "wait:Vendor".into(),
                "due:2025-02-01".into(),
            ],
            notes: None,
            project: None,
            areas: vec![],
            status: None,
            contexts: vec![],
            tags: vec![],
            due_at: None,
            defer_until: None,
            time_estimate: None,
            energy: None,
            priority: None,
            waiting_on: None,
            waiting_since: None,
        };

        db.handle_add(&args).expect("add waiting task");

        let filters = ListFilters::for_view(Some(ListView::Scheduled));
        let items = db.fetch_tasks(&filters).expect("fetch scheduled");
        assert!(
            items.is_empty(),
            "waiting task should not show in scheduled view"
        );
    }

    #[test]
    fn update_task_applies_new_details() {
        let (config, _dir) = temp_config();
        let mut db = Database::initialize(&config).expect("initialize db");

        let add_args = TaskInput {
            text: vec![
                "Write".into(),
                "docs".into(),
                "+Project".into(),
                "@home".into(),
            ],
            notes: Some("Initial notes".into()),
            project: None,
            areas: vec![],
            status: None,
            contexts: vec![],
            tags: vec![],
            due_at: None,
            defer_until: None,
            time_estimate: None,
            energy: None,
            priority: Some(1),
            waiting_on: None,
            waiting_since: None,
        };
        let outcome = db.handle_add(&add_args).expect("add task");
        let id = outcome.id;

        let parsed = parser::parse_capture(&TaskInput {
            text: vec![
                "Revise".into(),
                "plan".into(),
                "+Strategy".into(),
                "@office".into(),
                "#q4".into(),
                "p:2".into(),
            ],
            notes: None,
            project: None,
            areas: vec![],
            status: None,
            contexts: vec![],
            tags: vec![],
            due_at: None,
            defer_until: None,
            time_estimate: Some(30),
            energy: Some("med".into()),
            priority: None,
            waiting_on: None,
            waiting_since: None,
        })
        .expect("parse capture");

        let mut updated_task = parsed.task;
        updated_task.areas = vec!["focus".into()];

        let updated = db
            .update_task(&id, &updated_task)
            .expect("update")
            .expect("task exists");

        assert_eq!(updated.title, "Revise plan");
        assert_eq!(updated.project.as_deref(), Some("Strategy"));
        assert_eq!(updated.contexts, vec!["office".to_string()]);
        assert_eq!(updated.tags, vec!["q4".to_string()]);
        assert_eq!(updated.priority, 2);
        assert_eq!(updated.time_estimate, Some(30));
        assert_eq!(updated.energy.map(|e| e.as_str()), Some("med"));
        assert_eq!(updated.areas, vec!["focus".to_string()]);
    }

    #[test]
    fn project_summaries_track_status_counts() {
        let (config, _dir) = temp_config();
        let mut db = Database::initialize(&config).expect("initialize db");

        let seed = |status: TaskStatus| TaskInput {
            text: vec![format!("{} task", status.as_str()), "+Alpha".into()],
            notes: None,
            project: None,
            areas: vec![],
            status: Some(status),
            contexts: vec![],
            tags: vec![],
            due_at: None,
            defer_until: None,
            time_estimate: None,
            energy: None,
            priority: None,
            waiting_on: None,
            waiting_since: None,
        };

        db.handle_add(&seed(TaskStatus::Next)).unwrap();
        db.handle_add(&seed(TaskStatus::Waiting)).unwrap();
        db.handle_add(&seed(TaskStatus::Someday)).unwrap();

        let filters = ListFilters::for_view(Some(ListView::Projects));
        let items = db.fetch_tasks(&filters).expect("project summary");
        assert_eq!(items.len(), 1);
        let summary = match &items[0] {
            ListOutputItem::Project(project) => project,
            _ => panic!("expected project summary"),
        };
        assert_eq!(summary.project, "Alpha");
        assert_eq!(summary.total, 3);
        assert_eq!(summary.next_actions, 1);
        assert_eq!(summary.waiting, 1);
        assert_eq!(summary.someday, 1);
    }
}
