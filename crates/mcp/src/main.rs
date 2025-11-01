use std::path::PathBuf;

use clap::Parser;
use cpt_mcp::{run_server, ServerConfig};

#[derive(Parser, Debug)]
#[command(
    name = "cpt-mcp",
    version,
    about = "Model Context Protocol server for the cpt.run CLI"
)]
struct Args {
    /// Override the cpt.run data directory (defaults to the same resolution as the TUI)
    #[arg(long = "data-dir", value_name = "PATH")]
    data_dir: Option<PathBuf>,

    /// Override the tracing filter (e.g. "info", "debug", or full directives)
    #[arg(long = "log", value_name = "DIRECTIVE")]
    log_filter: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let config = ServerConfig {
        data_dir: args.data_dir,
        log_filter: args.log_filter,
    };

    run_server(config).await
}
