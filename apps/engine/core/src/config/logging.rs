//! Log level, output format and per-subsystem switches.

use serde::{Deserialize, Serialize};

/// Most verbose level of event that is emitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Errors only.
    Error,
    /// Warnings and errors.
    Warn,
    /// Informational events and above.
    #[default]
    Info,
    /// Debug events and above.
    Debug,
    /// Every event.
    Trace,
}

/// What the process logs and how it is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct LoggingConfig {
    /// Install a log subscriber. Off emits nothing.
    #[serde(default = "crate::config::default_true")]
    pub enabled: bool,
    /// Base level. RUST_LOG replaces it when set.
    #[serde(default)]
    pub level: LogLevel,
    /// Emit events on the piramid::config target.
    #[serde(default = "crate::config::default_true")]
    pub config: bool,
    /// Emit events on the piramid::indexing target.
    #[serde(default = "crate::config::default_true")]
    pub indexing: bool,
    /// Emit events on the piramid::search target.
    #[serde(default = "crate::config::default_true")]
    pub search: bool,
    /// Emit events on the piramid::writes target.
    #[serde(default = "crate::config::default_true")]
    pub writes: bool,
    /// Emit events on the piramid::inference target.
    #[serde(default = "crate::config::default_true")]
    pub inference: bool,
    /// Emit events on the piramid::http target.
    #[serde(default = "crate::config::default_true")]
    pub http: bool,
    /// Emit structured JSON lines instead of human-readable console output.
    #[serde(default)]
    pub json: bool,
    /// Queries slower than this many milliseconds are logged at warn. None uses 500.
    #[serde(default)]
    pub slow_query_ms: Option<u64>,
}

impl LoggingConfig {
    /// Threshold above which a query is logged at warn.
    pub fn slow_query_ms(&self) -> u64 {
        self.slow_query_ms.unwrap_or(DEFAULT_SLOW_QUERY_MS)
    }
}

/// The serde default, and the value used for an explicit null.
const DEFAULT_SLOW_QUERY_MS: u64 = 500;

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            level: LogLevel::Info,
            config: true,
            indexing: true,
            search: true,
            writes: true,
            inference: true,
            http: true,
            json: false,
            slow_query_ms: Some(DEFAULT_SLOW_QUERY_MS),
        }
    }
}
