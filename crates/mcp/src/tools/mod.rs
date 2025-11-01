mod capture;
mod defer_task;
mod delete;
mod get;
mod list;
mod status;
mod util;

use std::sync::Arc;

use cpt_core::services::TasksService;
use pmcp::ServerBuilder;

pub fn register(builder: ServerBuilder, service: Arc<TasksService>) -> ServerBuilder {
    builder
        .tool("list_tasks", list::ListTasksTool::new(service.clone()))
        .tool("get_task", get::GetTaskTool::new(service.clone()))
        .tool(
            "capture_task",
            capture::CaptureTaskTool::new(service.clone()),
        )
        .tool(
            "set_task_status",
            status::SetTaskStatusTool::new(service.clone()),
        )
        .tool(
            "defer_task",
            defer_task::DeferTaskTool::new(service.clone()),
        )
        .tool("delete_tasks", delete::DeleteTasksTool::new(service))
}
