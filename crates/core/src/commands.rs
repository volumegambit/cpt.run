use anyhow::Result;

use crate::config::AppConfig;
use crate::database::Database;
use crate::model::DeleteResult;

/// Delete the tasks with the provided ids and return per-id results.
pub fn delete_tasks(config: &AppConfig, ids: &[String]) -> Result<Vec<DeleteResult>> {
    let database = Database::initialize(config)?;
    database.delete_tasks(ids)
}
