use std::sync::Arc;

use anyhow::{anyhow, Result};
use cpt_core::services::TasksService;

pub async fn with_service<T, F>(service: Arc<TasksService>, f: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce(&TasksService) -> Result<T> + Send + 'static,
{
    tokio::task::spawn_blocking(move || f(service.as_ref()))
        .await
        .map_err(|err| anyhow!("blocking task failed: {}", err))?
}

pub fn validation_error(err: impl std::fmt::Display) -> pmcp::Error {
    pmcp::Error::validation(err.to_string())
}

pub fn internal_error(err: impl Into<anyhow::Error>) -> pmcp::Error {
    pmcp::Error::internal(err.into().to_string())
}

#[cfg(test)]
pub(crate) fn test_service() -> (Arc<TasksService>, tempfile::TempDir) {
    use cpt_core::config::AppConfig;

    let dir = tempfile::TempDir::new().expect("temp dir");
    let config = AppConfig::from_data_dir(dir.path().to_path_buf()).expect("config");
    let service = TasksService::new(config).expect("service");
    (Arc::new(service), dir)
}

#[cfg(test)]
pub(crate) fn test_extra() -> pmcp::RequestHandlerExtra {
    pmcp::RequestHandlerExtra::new(
        "test-request".to_string(),
        tokio_util::sync::CancellationToken::new(),
    )
}
