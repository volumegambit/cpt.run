mod tools;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use cpt_core::config::AppConfig;
use cpt_core::services::TasksService;
use pmcp::types::capabilities::ServerCapabilities;
use pmcp::Server;
use tracing_subscriber::filter::Directive;
use tracing_subscriber::EnvFilter;

/// Runtime configuration for the cpt.run MCP server.
#[derive(Debug, Clone, Default)]
pub struct ServerConfig {
    pub data_dir: Option<PathBuf>,
    pub log_filter: Option<String>,
}

/// Launch the MCP server using the provided configuration.
pub async fn run_server(config: ServerConfig) -> Result<()> {
    init_tracing(config.log_filter.clone())?;

    let app_config =
        AppConfig::discover(config.data_dir.clone()).context("failed to resolve data directory")?;
    let tasks_service = Arc::new(
        TasksService::new(app_config.clone()).context("failed to initialize task service")?,
    );

    let server = build_server(tasks_service.clone()).context("failed to build MCP server")?;

    eprintln!(
        "Starting cpt-mcp v{} (data dir: {}) with tools: capture_task, list_tasks, update_status, defer_task, delete_tasks",
        env!("CARGO_PKG_VERSION"),
        app_config.data_dir().display()
    );

    server
        .run_stdio()
        .await
        .map_err(|err| anyhow::anyhow!("MCP server error: {}", err))
}

/// Run the MCP server by creating an internal Tokio runtime.
pub fn run_server_blocking(config: ServerConfig) -> Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to build tokio runtime")?;
    runtime.block_on(run_server(config))
}

fn init_tracing(filter: Option<String>) -> Result<()> {
    let filter = filter.unwrap_or_else(|| "info".to_string());
    let directive: Directive = filter.parse()?;
    let env_filter = EnvFilter::builder()
        .with_default_directive(directive)
        .from_env_lossy();

    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .compact()
        .try_init();
    Ok(())
}

fn build_server(service: Arc<TasksService>) -> Result<Server> {
    let builder = Server::builder()
        .name("cpt-mcp")
        .version(env!("CARGO_PKG_VERSION"))
        .capabilities(ServerCapabilities::tools_only());

    let builder = tools::register(builder, service);
    builder
        .build()
        .map_err(|err| anyhow::anyhow!(err.to_string()))
}
