use std::time::Duration;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = cpt::cli::Cli::parse();

    match cli.command.clone() {
        Some(cpt::cli::CliCommand::Desktop(args)) => {
            let options = cpt::DesktopOptions {
                data_dir: cli.data_dir.clone(),
                refresh_interval: Duration::from_secs(args.refresh_interval),
            };
            cpt::desktop::run(options)?;
        }
        Some(cpt::cli::CliCommand::Tui) | None => {
            let config = cpt::config::from_cli(&cli)?;
            cpt::tui::run(config)?;
        }
        Some(cpt::cli::CliCommand::Mcp(args)) => {
            let config = cpt::mcp::ServerConfig {
                data_dir: cli.data_dir.clone(),
                log_filter: args.log_filter.clone(),
            };
            cpt::mcp::run_server_blocking(config)?;
        }
        Some(command) => {
            let config = cpt::config::from_cli(&cli)?;
            let stdout = std::io::stdout();
            let mut handle = stdout.lock();
            cpt::commands::execute(&config, command, &mut handle)?;
        }
    }

    Ok(())
}
