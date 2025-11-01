use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use directories::{BaseDirs, ProjectDirs};
use once_cell::sync::Lazy;

static DEFAULT_DB_NAME: &str = "cpt.sqlite3";
static ENV_DATA_DIR: &str = "CPT_DATA_DIR";

static PROJECT_DIRS: Lazy<Option<ProjectDirs>> =
    Lazy::new(|| ProjectDirs::from("dev", "cpt-cli", "cpt"));

#[derive(Debug, Clone)]
pub struct AppConfig {
    data_dir: PathBuf,
    db_path: PathBuf,
}

impl AppConfig {
    /// Construct [`AppConfig`] by resolving the data directory using the provided override,
    /// environment variables, and platform defaults.
    pub fn discover(data_dir_override: Option<PathBuf>) -> Result<Self> {
        let data_dir = resolve_data_dir(data_dir_override)?;
        if !data_dir.exists() {
            fs::create_dir_all(&data_dir).with_context(|| {
                format!("Failed to create data directory at {}", data_dir.display())
            })?;
        }
        Self::from_data_dir(data_dir)
    }

    /// Construct [`AppConfig`] directly from a resolved data directory.
    pub fn from_data_dir(data_dir: PathBuf) -> Result<Self> {
        let db_path = data_dir.join(DEFAULT_DB_NAME);
        Ok(Self { data_dir, db_path })
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }
}

fn resolve_data_dir(data_dir_override: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(dir) = data_dir_override {
        return Ok(dir);
    }

    if let Ok(env_dir) = env::var(ENV_DATA_DIR) {
        return Ok(PathBuf::from(env_dir));
    }

    if cfg!(debug_assertions) {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let dev_dir = manifest_dir.join("..").join("tmp").join("dev-cpt");
        return Ok(dev_dir);
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(base) = BaseDirs::new() {
            return Ok(base.home_dir().join(".cpt"));
        }
    }

    if let Some(project) = &*PROJECT_DIRS {
        return Ok(project.data_dir().to_path_buf());
    }

    if let Some(base) = BaseDirs::new() {
        return Ok(base.home_dir().join(".cpt"));
    }

    Ok(env::current_dir()?.join(".cpt"))
}
