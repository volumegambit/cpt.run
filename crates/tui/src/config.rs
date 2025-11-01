pub use cpt_core::config::*;

use crate::cli::Cli;

pub fn from_cli(cli: &Cli) -> anyhow::Result<AppConfig> {
    AppConfig::discover(cli.data_dir.clone())
}
