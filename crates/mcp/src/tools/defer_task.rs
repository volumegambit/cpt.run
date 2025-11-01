use std::sync::Arc;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use cpt_core::services::TasksService;
use pmcp::{RequestHandlerExtra, Result as McpResult, ToolHandler};
use serde::Deserialize;
use serde_json::{json, Value};

use super::util::{internal_error, validation_error, with_service};

pub struct DeferTaskTool {
    service: Arc<TasksService>,
}

impl DeferTaskTool {
    pub fn new(service: Arc<TasksService>) -> Self {
        Self { service }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeferTaskArgs {
    id: String,
    defer_until: Option<String>,
}

#[async_trait]
impl ToolHandler for DeferTaskTool {
    async fn handle(&self, args: Value, _extra: RequestHandlerExtra) -> McpResult<Value> {
        let parsed: DeferTaskArgs =
            serde_json::from_value(args).map_err(|err| validation_error(err))?;

        let id = parsed.id;
        let defer_until = parse_defer(parsed.defer_until.as_deref()).map_err(validation_error)?;

        let updated = with_service(self.service.clone(), move |service| {
            service.defer_until(&id, defer_until)
        })
        .await
        .map_err(internal_error)?;

        Ok(json!({ "task": updated }))
    }

    fn metadata(&self) -> Option<pmcp::types::ToolInfo> {
        Some(pmcp::types::ToolInfo {
            name: "defer_task".to_string(),
            description: Some("Set or clear a task's defer_until timestamp".to_string()),
            input_schema: json!({
                "type": "object",
                "required": ["id"],
                "properties": {
                    "id": { "type": "string" },
                    "deferUntil": {
                        "type": ["string", "null"],
                        "description": "RFC3339 timestamp; omit or null to clear defer date"
                    }
                }
            }),
        })
    }
}

fn parse_defer(value: Option<&str>) -> Result<Option<DateTime<Utc>>> {
    match value {
        Some(raw) => {
            let dt = DateTime::parse_from_rfc3339(raw)
                .map_err(|err| anyhow!("invalid RFC3339 timestamp: {}", err))?;
            Ok(Some(dt.with_timezone(&Utc)))
        }
        None => Ok(None),
    }
}
