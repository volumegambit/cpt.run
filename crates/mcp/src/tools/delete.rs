use std::sync::Arc;

use async_trait::async_trait;
use cpt_core::services::TasksService;
use pmcp::{RequestHandlerExtra, Result as McpResult, ToolHandler};
use serde::Deserialize;
use serde_json::{json, Value};

use super::util::{internal_error, validation_error, with_service};

pub struct DeleteTasksTool {
    service: Arc<TasksService>,
}

impl DeleteTasksTool {
    pub fn new(service: Arc<TasksService>) -> Self {
        Self { service }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteTasksArgs {
    ids: Vec<String>,
}

#[async_trait]
impl ToolHandler for DeleteTasksTool {
    async fn handle(&self, args: Value, _extra: RequestHandlerExtra) -> McpResult<Value> {
        let parsed: DeleteTasksArgs =
            serde_json::from_value(args).map_err(|err| validation_error(err))?;
        if parsed.ids.is_empty() {
            return Err(validation_error("ids must contain at least one task id"));
        }

        let ids = parsed.ids;
        let results = with_service(self.service.clone(), move |service| {
            service.delete_tasks(&ids)
        })
        .await
        .map_err(internal_error)?;

        Ok(json!({ "results": results }))
    }

    fn metadata(&self) -> Option<pmcp::types::ToolInfo> {
        Some(pmcp::types::ToolInfo {
            name: "delete_tasks".to_string(),
            description: Some("Delete one or more tasks by ULID".to_string()),
            input_schema: json!({
                "type": "object",
                "required": ["ids"],
                "properties": {
                    "ids": {
                        "type": "array",
                        "items": { "type": "string" },
                        "minItems": 1
                    }
                }
            }),
        })
    }
}
