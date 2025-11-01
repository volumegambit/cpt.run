use std::sync::Arc;

use async_trait::async_trait;
use cpt_core::services::TasksService;
use pmcp::{RequestHandlerExtra, Result as McpResult, ToolHandler};
use serde::Deserialize;
use serde_json::{json, Value};

use super::util::{internal_error, validation_error, with_service};

pub struct GetTaskTool {
    service: Arc<TasksService>,
}

impl GetTaskTool {
    pub fn new(service: Arc<TasksService>) -> Self {
        Self { service }
    }
}

#[derive(Debug, Deserialize)]
struct GetTaskArgs {
    id: String,
}

#[async_trait]
impl ToolHandler for GetTaskTool {
    async fn handle(&self, args: Value, _extra: RequestHandlerExtra) -> McpResult<Value> {
        let parsed: GetTaskArgs =
            serde_json::from_value(args).map_err(|err| validation_error(err))?;
        let id = parsed.id;
        let task = with_service(self.service.clone(), move |service| service.fetch_task(&id))
            .await
            .map_err(internal_error)?;

        Ok(json!({ "task": task }))
    }

    fn metadata(&self) -> Option<pmcp::types::ToolInfo> {
        Some(pmcp::types::ToolInfo {
            name: "get_task".to_string(),
            description: Some("Lookup a task by ULID".to_string()),
            input_schema: json!({
                "type": "object",
                "required": ["id"],
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Task ULID"
                    }
                }
            }),
        })
    }
}
