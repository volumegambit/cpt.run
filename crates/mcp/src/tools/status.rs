use std::sync::Arc;

use async_trait::async_trait;
use cpt_core::services::TasksService;
use pmcp::{RequestHandlerExtra, Result as McpResult, ToolHandler};
use serde::Deserialize;
use serde_json::{json, Value};

use super::util::{internal_error, validation_error, with_service};

pub struct SetTaskStatusTool {
    service: Arc<TasksService>,
}

impl SetTaskStatusTool {
    pub fn new(service: Arc<TasksService>) -> Self {
        Self { service }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StatusAction {
    PromoteToNext,
    MarkDone,
    MoveToInbox,
    MarkSomeday,
}

#[cfg(test)]
mod tests {
    use super::*;
    use cpt_core::capture::CaptureInput;
    use cpt_core::model::TaskStatus;
    use serde_json::json;

    use crate::tools::util::{test_extra, test_service};

    #[tokio::test]
    async fn mark_done_updates_status() {
        let (service, _dir) = test_service();
        let id = {
            let mut input = CaptureInput::default();
            input.text = vec!["Review".into(), "spec".into()];
            service.capture(input).expect("capture").id
        };

        let tool = SetTaskStatusTool::new(service.clone());
        let response = tool
            .handle(
                json!({
                    "action": "mark_done",
                    "ids": [id.clone()]
                }),
                test_extra(),
            )
            .await
            .expect("status change");

        let updates = response["updates"].as_array().expect("updates array");
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0]["id"].as_str(), Some(id.as_str()));

        let updated = service.fetch_task(&id).expect("fetch").expect("task");
        assert_eq!(updated.status, TaskStatus::Done);
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetStatusArgs {
    action: StatusAction,
    ids: Vec<String>,
}

#[async_trait]
impl ToolHandler for SetTaskStatusTool {
    async fn handle(&self, args: Value, _extra: RequestHandlerExtra) -> McpResult<Value> {
        let parsed: SetStatusArgs =
            serde_json::from_value(args).map_err(|err| validation_error(err))?;

        if parsed.ids.is_empty() {
            return Err(validation_error("ids must contain at least one task id"));
        }

        let ids = parsed.ids;
        let service = self.service.clone();
        let action = parsed.action;
        let updates = with_service(service, move |service| match action {
            StatusAction::PromoteToNext => service.promote_to_next(&ids),
            StatusAction::MarkDone => service.mark_done(&ids),
            StatusAction::MoveToInbox => service.move_to_inbox(&ids),
            StatusAction::MarkSomeday => service.mark_someday(&ids),
        })
        .await
        .map_err(internal_error)?;

        Ok(json!({ "updates": updates }))
    }

    fn metadata(&self) -> Option<pmcp::types::ToolInfo> {
        Some(pmcp::types::ToolInfo {
            name: "set_task_status".to_string(),
            description: Some(
                "Update task status using the same transitions exposed in the TUI".to_string(),
            ),
            input_schema: json!({
                "type": "object",
                "required": ["action", "ids"],
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": [
                            "promote_to_next",
                            "mark_done",
                            "move_to_inbox",
                            "mark_someday"
                        ]
                    },
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
