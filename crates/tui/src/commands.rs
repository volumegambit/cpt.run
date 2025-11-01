use std::fmt;
use std::io::Write;

use anyhow::{anyhow, Result};

use crate::cli::{CliCommand, DeleteArgs};
use crate::config::AppConfig;
use crate::core::commands as core_commands;
use crate::model::DeleteResult;

pub fn execute<W: Write>(config: &AppConfig, command: CliCommand, mut writer: W) -> Result<()> {
    match command {
        CliCommand::Delete(args) => handle_delete(config, &args, &mut writer),
        CliCommand::Tui | CliCommand::Desktop(_) | CliCommand::Mcp(_) => {
            Err(anyhow!("launch interactive surfaces directly"))
        }
    }
}

fn handle_delete<W: Write>(config: &AppConfig, args: &DeleteArgs, mut writer: W) -> Result<()> {
    let results = core_commands::delete_tasks(config, &args.ids)?;
    let summary = DeleteSummary::from_results(&results);
    summary.write_to(&mut writer)?;
    Ok(())
}

struct DeleteSummary {
    deleted: usize,
    missing: Vec<String>,
}

impl DeleteSummary {
    fn from_results(results: &[DeleteResult]) -> Self {
        let mut deleted = 0usize;
        let mut missing = Vec::new();
        for result in results {
            if result.deleted {
                deleted += 1;
            } else {
                missing.push(result.id.clone());
            }
        }
        Self { deleted, missing }
    }

    fn write_to<W: Write>(&self, mut writer: W) -> Result<()> {
        writeln!(writer, "{}", SummaryLine::deleted(self.deleted))?;
        if !self.missing.is_empty() {
            writeln!(writer, "Not found: {}", self.missing.join(", "))?;
        }
        Ok(())
    }
}

enum SummaryLine {
    Deleted(usize),
    NoneDeleted,
}

impl SummaryLine {
    fn deleted(count: usize) -> Self {
        if count > 0 {
            SummaryLine::Deleted(count)
        } else {
            SummaryLine::NoneDeleted
        }
    }
}

impl fmt::Display for SummaryLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SummaryLine::Deleted(count) => {
                write!(
                    f,
                    "Deleted {} task{}",
                    count,
                    if *count == 1 { "" } else { "s" }
                )
            }
            SummaryLine::NoneDeleted => write!(f, "No tasks deleted"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::CaptureInput;
    use crate::db::Database;
    use tempfile::TempDir;

    fn temp_config() -> (AppConfig, TempDir) {
        let dir = TempDir::new().expect("temp dir");
        let data_dir = dir.path().to_path_buf();
        std::fs::create_dir_all(&data_dir).expect("create data dir");
        let config = AppConfig::from_data_dir(data_dir).expect("config");
        (config, dir)
    }

    fn seed_task(db: &mut Database, text: Vec<String>) -> String {
        let input = CaptureInput {
            text,
            notes: None,
            project: None,
            areas: vec![],
            status: None,
            contexts: vec![],
            tags: vec![],
            due_at: None,
            defer_until: None,
            time_estimate: None,
            energy: None,
            priority: None,
            waiting_on: None,
            waiting_since: None,
        };
        db.handle_add(&input).expect("add task").id
    }

    #[test]
    fn delete_command_reports_deleted_and_missing() {
        let (config, _dir) = temp_config();
        let task_id = {
            let mut db = Database::initialize(&config).expect("init db");
            seed_task(&mut db, vec!["Test".into()])
        };

        let args = DeleteArgs {
            ids: vec![task_id.clone(), "missing".into()],
        };
        let mut output = Vec::new();
        execute(&config, CliCommand::Delete(args), &mut output).expect("execute delete");
        let output = String::from_utf8(output).expect("utf8");

        assert!(output.contains("Deleted 1 task"));
        assert!(output.contains("Not found: missing"));
    }

    #[test]
    fn delete_command_handles_no_matches() {
        let (config, _dir) = temp_config();
        let args = DeleteArgs {
            ids: vec!["missing".into()],
        };
        let mut output = Vec::new();
        execute(&config, CliCommand::Delete(args), &mut output).expect("execute delete");
        let output = String::from_utf8(output).expect("utf8");

        assert!(output.contains("No tasks deleted"));
    }
}
