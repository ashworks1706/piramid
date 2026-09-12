//! Settings for the terminal UI.

use serde::{Deserialize, Serialize};

/// What the console watches and how often.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ConsoleConfig {
    /// Server to watch. Empty means the address [StartupConfig::bind] names.
    ///
    /// [StartupConfig::bind]: super::StartupConfig::bind
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
    /// The server to watch, falling back to the address the server binds.
    ///
    /// A bind of 0.0.0.0 is every interface, which is not an address to connect to, so the
    /// loopback address is used with the port it names.
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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "a failed assertion is the point of a test"
)]
mod tests {
    use super::*;

    #[test]
    fn an_unset_base_url_follows_the_address_the_server_binds() {
        let config = ConsoleConfig::default();

        // Nothing listens on 0.0.0.0, so the console connects to loopback on the same port.
        assert_eq!(
            config.resolved_base_url("0.0.0.0:6333"),
            "http://localhost:6333"
        );
        assert_eq!(
            config.resolved_base_url("127.0.0.1:7000"),
            "http://127.0.0.1:7000"
        );
    }

    #[test]
    fn a_set_base_url_wins_so_a_remote_server_can_be_watched() {
        let config = ConsoleConfig {
            base_url: "https://piramid.internal:6333".into(),
            ..ConsoleConfig::default()
        };

        assert_eq!(
            config.resolved_base_url("0.0.0.0:6333"),
            "https://piramid.internal:6333"
        );
    }

    #[test]
    fn zero_valued_knobs_are_refused() {
        let no_lines = ConsoleConfig {
            log_lines: 0,
            ..ConsoleConfig::default()
        };
        assert!(no_lines.validate().is_err());

        let no_interval = ConsoleConfig {
            refresh_secs: 0,
            ..ConsoleConfig::default()
        };
        assert!(no_interval.validate().is_err());
    }
}
