use std::path::PathBuf;

use clap::{value_parser, ArgAction, Args, Parser, Subcommand};

use crate::capture::CaptureInput;
use crate::model::TaskStatus;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "cpt",
    version,
    about = "A local-first privacy-first tool to get things done.",
    author = "Gerry Eng",
    after_help = "Examples:\n  cpt             Launch the TUI (same as `cpt tui`)\n  cpt desktop --refresh-interval 10\n  cpt mcp --log debug\n  cpt delete 123"
)]
pub struct Cli {
    /// Override the data directory (defaults to platform-specific app dir)
    #[arg(long, value_name = "PATH", global = true)]
    pub data_dir: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<CliCommand>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum CliCommand {
    /// Launch the keyboard-first terminal UI (default command)
    Tui,
    /// Launch the iced-based desktop shell
    Desktop(DesktopArgs),
    /// Delete one or more tasks by id
    Delete(DeleteArgs),
    /// Run the Model Context Protocol server over stdio
    Mcp(McpArgs),
}

#[derive(Args, Debug, Clone)]
pub struct DesktopArgs {
    /// Refresh interval (seconds) for background view updates
    #[arg(long = "refresh-interval", value_name = "SECONDS", default_value_t = 5, value_parser = value_parser!(u64))]
    pub refresh_interval: u64,
}

#[derive(Args, Debug, Clone)]
pub struct McpArgs {
    /// Override the tracing filter for the MCP server (e.g. "info", "debug")
    #[arg(long = "log", value_name = "DIRECTIVE")]
    pub log_filter: Option<String>,
}

#[derive(Args, Debug, Clone)]
pub struct AddArgs {
    /// Task title with optional inline tokens (@context, +project, #tag, due:, defer:, t:, e:, p:)
    #[arg(value_name = "TEXT", required = true)]
    pub text: Vec<String>,

    /// Optional detailed notes
    #[arg(long)]
    pub notes: Option<String>,

    /// Explicitly set the project (overrides inline +project token)
    #[arg(long)]
    pub project: Option<String>,

    /// Associate areas (comma-separated or repeated flag)
    #[arg(long, value_delimiter = ',', action = ArgAction::Append)]
    pub area: Vec<String>,

    /// Set status explicitly (defaults to inbox)
    #[arg(long, value_enum)]
    pub status: Option<TaskStatus>,

    /// Add contexts (comma-separated or repeated flag; '@' prefix optional)
    #[arg(long, value_delimiter = ',', action = ArgAction::Append)]
    pub context: Vec<String>,

    /// Add tags (comma-separated or repeated flag; '#' prefix optional)
    #[arg(long, value_delimiter = ',', action = ArgAction::Append)]
    pub tag: Vec<String>,

    /// Set due date (ISO e.g. 2023-12-24, today, +3d, mon)
    #[arg(long = "due", value_name = "DATE")]
    pub due_at: Option<String>,

    /// Set defer date (ISO, +Nd, tomorrow, etc.)
    #[arg(long = "defer", value_name = "DATE")]
    pub defer_until: Option<String>,

    /// Time estimate (minutes)
    #[arg(long = "time", value_parser = value_parser!(u32))]
    pub time_estimate: Option<u32>,

    /// Energy requirement (low, med, high)
    #[arg(long = "energy", value_name = "LEVEL")]
    pub energy: Option<String>,

    /// Priority (0-3)
    #[arg(long = "priority", value_parser = value_parser!(u8))]
    pub priority: Option<u8>,

    /// Person or contact the task is waiting on
    #[arg(long = "waiting-on")]
    pub waiting_on: Option<String>,

    /// Set status to waiting and capture since timestamp (ISO or relative)
    #[arg(long = "waiting-since")]
    pub waiting_since: Option<String>,
}

#[derive(Args, Debug, Clone)]
pub struct DeleteArgs {
    /// One or more task ids to delete (use `/delete` or `x` in the TUI to copy ids)
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
}

impl From<&AddArgs> for CaptureInput {
    fn from(args: &AddArgs) -> Self {
        CaptureInput {
            text: args.text.clone(),
            notes: args.notes.clone(),
            project: args.project.clone(),
            areas: args.area.clone(),
            status: args.status,
            contexts: args.context.clone(),
            tags: args.tag.clone(),
            due_at: args.due_at.clone(),
            defer_until: args.defer_until.clone(),
            time_estimate: args.time_estimate,
            energy: args.energy.clone(),
            priority: args.priority,
            waiting_on: args.waiting_on.clone(),
            waiting_since: args.waiting_since.clone(),
        }
    }
}

impl From<AddArgs> for CaptureInput {
    fn from(args: AddArgs) -> Self {
        CaptureInput {
            text: args.text,
            notes: args.notes,
            project: args.project,
            areas: args.area,
            status: args.status,
            contexts: args.context,
            tags: args.tag,
            due_at: args.due_at,
            defer_until: args.defer_until,
            time_estimate: args.time_estimate,
            energy: args.energy,
            priority: args.priority,
            waiting_on: args.waiting_on,
            waiting_since: args.waiting_since,
        }
    }
}
