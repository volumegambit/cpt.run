//! Configuration surfaces for tailoring the desktop shell to a user's workspace.

use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct DesktopOptions {
    pub data_dir: Option<PathBuf>,
    pub refresh_interval: Duration,
}

impl Default for DesktopOptions {
    fn default() -> Self {
        Self {
            data_dir: None,
            refresh_interval: Duration::from_secs(5),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct DesktopFlags {
    pub(crate) data_dir: Option<PathBuf>,
    pub(crate) refresh_interval: Duration,
}

impl From<DesktopOptions> for DesktopFlags {
    fn from(options: DesktopOptions) -> Self {
        Self {
            data_dir: options.data_dir,
            refresh_interval: options.refresh_interval,
        }
    }
}
