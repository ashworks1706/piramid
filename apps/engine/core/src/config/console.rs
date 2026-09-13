//! Settings for the terminal UI.

use serde::{Deserialize, Serialize};

/// What the console watches and how often.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ConsoleConfig {
    /// Server to watch. Empty means the address [super::StartupConfig::bind] names.
    pub base_url: String,
    /// Website to probe, shown only inside a checkout.
    pub web_url: String,
    /// Lines kept in memory per unit.
    pub log_lines: usize,
    /// Directory for unit logs, relative to the working directory unless absolute.
    pub log_dir: String,
    /// Seconds between health probes and collection refreshes.
    pub refresh_secs: u64,
}

impl Default for ConsoleConfig {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            web_url: "http://localhost:3000".into(),
            log_lines: 5000,
            log_dir: "target/console-logs".into(),
            refresh_secs: 5,
        }
    }
}

impl ConsoleConfig {
    /// The server to watch: base_url when set, otherwise the address bind names.
    pub fn resolved_base_url(&self, bind: &str) -> String {
        if !self.base_url.is_empty() {
            return self.base_url.clone();
        }
        let port = bind.rsplit(':').next().unwrap_or("6333");
        let host = match bind.rsplit_once(':').map(|(host, _)| host) {
            Some("0.0.0.0" | "[::]" | "") | None => "localhost",
            Some(host) => host,
        };
        format!("http://{host}:{port}")
    }

    /// Reject a zero log line count or refresh interval.
    pub fn validate(&self) -> Result<(), String> {
        if self.log_lines == 0 {
            return Err("console.log_lines must be greater than zero".into());
        }
        if self.refresh_secs == 0 {
            return Err("console.refresh_secs must be greater than zero".into());
        }
        Ok(())
    }
}
