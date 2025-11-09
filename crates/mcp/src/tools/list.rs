use std::str::FromStr;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use cpt_core::model::{EnergyLevel, ListFilters, ListView, SortField, TaskStatus};
use cpt_core::services::{TasksService, ViewSnapshot};
use pmcp::{RequestHandlerExtra, Result as McpResult, ToolHandler};
use serde::Deserialize;
use serde_json::{json, Value};

use super::util::{internal_error, validation_error, with_service};

pub struct ListTasksTool {
    service: Arc<TasksService>,
}

impl ListTasksTool {
    pub fn new(service: Arc<TasksService>) -> Self {
        Self { service }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListTasksArgs {
    view: Option<String>,
    status: Option<String>,
    project: Option<String>,
    contexts: Option<Vec<String>>,
    tags: Option<Vec<String>>,
    due_before: Option<String>,
    defer_after: Option<String>,
    time_max: Option<u32>,
    energy: Option<String>,
    priority_min: Option<u8>,
    include_done: Option<bool>,
    sort: Option<String>,
    reverse: Option<bool>,
}

impl ListTasksArgs {
    fn to_filters(&self) -> Result<ListFilters> {
        let view = self
            .view
            .as_ref()
            .map(|value| parse_view(value))
            .transpose()?;

        let mut filters = ListFilters::for_view(view);

        if let Some(status) = &self.status {
            filters.status = Some(TaskStatus::from_str(status)?);
        }

        if let Some(project) = &self.project {
            let trimmed = project.trim();
            filters.project = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            };
        }

        if let Some(contexts) = &self.contexts {
            filters.contexts = normalize_vec(contexts);
        }

        if let Some(tags) = &self.tags {
            filters.tags = normalize_vec(tags);
        }

        filters.due_before = parse_datetime_opt(self.due_before.as_deref())?;
        filters.defer_after = parse_datetime_opt(self.defer_after.as_deref())?;
        filters.time_max = self.time_max;
        filters.priority_min = self.priority_min;

        if let Some(energy) = &self.energy {
            filters.energy = Some(EnergyLevel::from_str(energy)?);
        }

        if let Some(include_done) = self.include_done {
            filters.include_done = include_done;
        }

        if let Some(sort) = &self.sort {
            filters.sort = SortField::from_str(sort)?;
        }

        if let Some(reverse) = self.reverse {
            filters.reverse = reverse;
        }

        Ok(filters)
    }
}

#[async_trait]
impl ToolHandler for ListTasksTool {
    async fn handle(&self, args: Value, _extra: RequestHandlerExtra) -> McpResult<Value> {
        let parsed: ListTasksArgs =
            serde_json::from_value(args).map_err(|err| validation_error(err))?;

        let filters = parsed.to_filters().map_err(|err| validation_error(err))?;
        let filters_clone = filters.clone();
        let snapshot = with_service(self.service.clone(), move |service| {
            service.list(&filters_clone)
        })
        .await
        .map_err(internal_error)?;

        Ok(build_snapshot_response(snapshot))
    }

    fn metadata(&self) -> Option<pmcp::types::ToolInfo> {
        Some(pmcp::types::ToolInfo {
            name: "list_tasks".to_string(),
            description: Some("List GTD tasks matching a view and optional filters".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "view": {
                        "type": "string",
                        "enum": [
                            "inbox",
                            "next",
                            "waiting",
                            "scheduled",
                            "someday",
                            "projects",
                            "done"
                        ],
                        "description": "Optional GTD view to focus the list"
                    },
                    "status": {
                        "type": "string",
                        "description": "Override status filter (inbox|next|waiting|scheduled|someday|done|canceled)"
                    },
                    "project": { "type": "string" },
                    "contexts": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "tags": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "dueBefore": {
                        "type": "string",
                        "format": "date-time",
                        "description": "Only include tasks due before this timestamp"
                    },
                    "deferAfter": {
                        "type": "string",
                        "format": "date-time",
                        "description": "Only include tasks deferred after this timestamp"
                    },
                    "timeMax": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "Maximum time estimate in minutes"
                    },
                    "energy": {
                        "type": "string",
                        "enum": ["low", "med", "high"]
                    },
                    "priorityMin": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 3
                    },
                    "includeDone": { "type": "boolean" },
                    "sort": {
                        "type": "string",
                        "enum": ["due", "priority", "created"]
                    },
                    "reverse": { "type": "boolean" }
                }
            }),
        })
    }
}

fn build_snapshot_response(snapshot: ViewSnapshot) -> Value {
    let view = snapshot.view().map(view_to_str);
    let ViewSnapshot {
        filters,
        tasks,
        projects,
    } = snapshot;
    json!({
        "view": view,
        "tasks": tasks,
        "projects": projects,
        "filters": {
            "status": filters.status.map(|s| s.as_str().to_string()),
            "project": filters.project,
            "contexts": filters.contexts,
            "tags": filters.tags,
            "dueBefore": filters.due_before.map(|d| d.to_rfc3339()),
            "deferAfter": filters.defer_after.map(|d| d.to_rfc3339()),
            "timeMax": filters.time_max,
            "energy": filters.energy.map(|e| e.as_str().to_string()),
            "priorityMin": filters.priority_min,
            "includeDone": filters.include_done,
            "sort": sort_to_str(filters.sort),
            "reverse": filters.reverse,
        }
    })
}

fn normalize_vec(values: &[String]) -> Vec<String> {
    let mut normalized: Vec<String> = values
        .iter()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .collect();
    normalized.sort();
    normalized.dedup();
    normalized
}

fn parse_datetime_opt(value: Option<&str>) -> Result<Option<DateTime<Utc>>> {
    match value {
        Some(raw) => {
            let parsed = DateTime::parse_from_rfc3339(raw)
                .map_err(|_| anyhow!("invalid RFC3339 timestamp: {}", raw))?;
            Ok(Some(parsed.with_timezone(&Utc)))
        }
        None => Ok(None),
    }
}

fn parse_view(value: &str) -> Result<ListView> {
    match value.to_ascii_lowercase().as_str() {
        "inbox" => Ok(ListView::Inbox),
        "next" => Ok(ListView::Next),
        "waiting" => Ok(ListView::Waiting),
        "scheduled" => Ok(ListView::Scheduled),
        "someday" => Ok(ListView::Someday),
        "projects" => Ok(ListView::Projects),
        "done" => Ok(ListView::Done),
        other => Err(anyhow!(
            "Unknown view '{}': expected inbox|next|waiting|scheduled|someday|projects|done",
            other
        )),
    }
}

fn view_to_str(view: ListView) -> &'static str {
    match view {
        ListView::Inbox => "inbox",
        ListView::Next => "next",
        ListView::Waiting => "waiting",
        ListView::Scheduled => "scheduled",
        ListView::Someday => "someday",
        ListView::Projects => "projects",
        ListView::Done => "done",
    }
}

fn sort_to_str(sort: SortField) -> &'static str {
    match sort {
        SortField::Due => "due",
        SortField::Priority => "priority",
        SortField::Created => "created",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cpt_core::capture::TaskInput;
    use cpt_core::model::TaskStatus;
    use serde_json::json;

    use crate::tools::util::{test_extra, test_service};

    #[tokio::test]
    async fn lists_tasks_by_view() {
        let (service, _dir) = test_service();
        {
            let mut inbox = TaskInput::default();
            inbox.text = vec!["Draft".into(), "plan".into()];
            service.capture(inbox).expect("capture inbox");

            let mut next = TaskInput::default();
            next.text = vec!["Ship".into(), "feature".into()];
            next.status = Some(TaskStatus::Next);
            service.capture(next).expect("capture next");
        }

        let tool = ListTasksTool::new(service);
        let response = tool
            .handle(
                json!({
                    "view": "next"
                }),
                test_extra(),
            )
            .await
            .expect("list next view");

        let tasks = response["tasks"].as_array().expect("tasks array");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0]["status"].as_str(), Some("next"));
    }
}
