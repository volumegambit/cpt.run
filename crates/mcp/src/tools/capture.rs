use std::str::FromStr;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use cpt_core::capture::TaskInput;
use cpt_core::model::TaskStatus;
use cpt_core::services::TasksService;
use pmcp::{RequestHandlerExtra, Result as McpResult, ToolHandler};
use serde::Deserialize;
use serde_json::{json, Value};

use super::util::{internal_error, validation_error, with_service};

pub struct CaptureTaskTool {
    service: Arc<TasksService>,
}

impl CaptureTaskTool {
    pub fn new(service: Arc<TasksService>) -> Self {
        Self { service }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CaptureTaskArgs {
    text: String,
    notes: Option<String>,
    project: Option<String>,
    areas: Option<Vec<String>>,
    status: Option<String>,
    contexts: Option<Vec<String>>,
    tags: Option<Vec<String>>,
    due_at: Option<String>,
    defer_until: Option<String>,
    time_estimate: Option<u32>,
    energy: Option<String>,
    priority: Option<u8>,
    waiting_on: Option<String>,
    waiting_since: Option<String>,
}

impl CaptureTaskArgs {
    fn into_input(self) -> Result<TaskInput> {
        let status = match self.status {
            Some(status) => Some(TaskStatus::from_str(&status)?),
            None => None,
        };

        let energy = self
            .energy
            .map(|value| value.trim().to_string())
            .filter(|v| !v.is_empty());
        let contexts = normalize_list(self.contexts);
        let tags = normalize_list(self.tags);
        let areas = normalize_list(self.areas);

        Ok(TaskInput {
            text: tokenize(&self.text),
            notes: self.notes,
            project: self.project,
            areas,
            status,
            contexts,
            tags,
            due_at: self.due_at,
            defer_until: self.defer_until,
            time_estimate: self.time_estimate,
            energy,
            priority: self.priority,
            waiting_on: self.waiting_on,
            waiting_since: self.waiting_since,
        })
    }
}

#[async_trait]
impl ToolHandler for CaptureTaskTool {
    async fn handle(&self, args: Value, _extra: RequestHandlerExtra) -> McpResult<Value> {
        let parsed: CaptureTaskArgs =
            serde_json::from_value(args).map_err(|err| validation_error(err))?;
        let input = parsed.into_input().map_err(|err| validation_error(err))?;

        let outcome = with_service(self.service.clone(), move |service| service.capture(input))
            .await
            .map_err(validation_error)?;

        serde_json::to_value(outcome).map_err(internal_error)
    }

    fn metadata(&self) -> Option<pmcp::types::ToolInfo> {
        Some(pmcp::types::ToolInfo {
            name: "capture_task".to_string(),
            description: Some(
                "Capture a GTD task using the same fields supported by the CLI".to_string(),
            ),
            input_schema: json!({
                "type": "object",
                "required": ["text"],
                "properties": {
                    "text": {
                        "type": "string",
                        "description": "Task text including inline GTD tokens"
                    },
                    "notes": { "type": "string" },
                    "project": { "type": "string" },
                    "areas": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "status": {
                        "type": "string",
                        "enum": ["inbox", "next", "waiting", "scheduled", "someday", "done", "canceled"]
                    },
                    "contexts": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "tags": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "dueAt": { "type": "string", "description": "ISO-8601 timestamp" },
                    "deferUntil": { "type": "string", "description": "ISO-8601 timestamp" },
                    "timeEstimate": { "type": "integer", "minimum": 0 },
                    "energy": { "type": "string" },
                    "priority": { "type": "integer", "minimum": 0, "maximum": 3 },
                    "waitingOn": { "type": "string" },
                    "waitingSince": { "type": "string" }
                }
            }),
        })
    }
}

fn tokenize(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|part| part.to_string())
        .collect::<Vec<_>>()
}

fn normalize_list(list: Option<Vec<String>>) -> Vec<String> {
    list.unwrap_or_default()
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    use crate::tools::util::{test_extra, test_service};

    #[tokio::test]
    async fn capture_creates_task() {
        let (service, _dir) = test_service();
        let tool = CaptureTaskTool::new(service);

        let response = tool
            .handle(
                json!({
                    "text": "Write integration tests @dev",
                    "notes": "Cover MCP flow"
                }),
                test_extra(),
            )
            .await
            .expect("capture result");

        assert_eq!(response["title"].as_str(), Some("Write integration tests"));
    }
}
