//! Where the console looks, and the checkout it can drive.

use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::time::Duration;

use piramid_core::config::{ApiKey, Config, ConsoleConfig};

/// The console settings, resolved from the configuration file.
#[derive(Debug, Clone)]
pub struct Settings {
    /// Server to watch.
    pub base_url: String,
    /// Address the serve unit listens on, from the bind address of the configuration.
    pub serve_url: String,
    /// Website to probe.
    pub web_url: String,
    /// Lines kept in memory per unit.
    pub log_lines: NonZeroUsize,
    /// Directory for unit logs.
    pub log_dir: PathBuf,
    /// Time between probes and refreshes.
    pub refresh: Duration,
    /// Key sent to the server, from PIRAMID_API_KEY.
    pub api_key: Option<ApiKey>,
}

impl Settings {
    /// Settings from a loaded configuration.
    ///
    /// Reads the same file and the same environment overrides as the server.
    ///
    /// Returns an error if the console section fails validation.
    pub fn from_config(config: &Config) -> Result<Self, String> {
        let console = &config.console;
        console.validate()?;
        let log_lines = NonZeroUsize::new(console.log_lines)
            .ok_or_else(|| "console.log_lines must be greater than zero".to_owned())?;
        // The bind address alone, with console.base_url cleared.
        let serve_url = ConsoleConfig {
            base_url: String::new(),
            ..console.clone()
        }
        .resolved_base_url(&config.startup.bind);
        Ok(Self {
            base_url: console.resolved_base_url(&config.startup.bind),
            serve_url,
            web_url: console.web_url.clone(),
            log_lines,
            log_dir: PathBuf::from(&console.log_dir),
            refresh: Duration::from_secs(console.refresh_secs),
            api_key: config.startup.http.auth.api_key.clone(),
        })
    }

    /// The log directory as an absolute path under root.
    pub fn log_dir_under(&self, root: &Path) -> PathBuf {
        if self.log_dir.is_absolute() {
            self.log_dir.clone()
        } else {
            root.join(&self.log_dir)
        }
    }
}

/// Walks up from start to the directory holding the justfile of the repo.
///
/// Returns None outside a checkout, where there is no justfile to drive.
pub fn repo_root(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|dir| dir.join("justfile").is_file() && dir.join("apps").is_dir())
        .map(Path::to_path_buf)
}
