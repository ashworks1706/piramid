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
    /// Base level.
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
    /// Emit structured JSON lines. Off writes human-readable console output.
    #[serde(default)]
    pub json: bool,
    /// Threshold in milliseconds above which a query is logged at warn.
    #[serde(default = "default_slow_query_ms")]
    pub slow_query_ms: u64,
}

fn default_slow_query_ms() -> u64 {
    500
}

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
            slow_query_ms: default_slow_query_ms(),
        }
    }
}
