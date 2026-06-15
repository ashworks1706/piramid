use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub level: LogLevel,
    #[serde(default = "default_true")]
    pub config: bool,
    #[serde(default = "default_true")]
    pub indexing: bool,
    #[serde(default = "default_true")]
    pub search: bool,
    #[serde(default = "default_true")]
    pub writes: bool,
    #[serde(default = "default_true")]
    pub inference: bool,
    #[serde(default)]
    pub slow_query_ms: Option<u64>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            level: LogLevel::Info,
            config: true,
            indexing: true,
            search: true,
            writes: true,
            inference: true,
            slow_query_ms: Some(500),
        }
    }
}

fn default_enabled() -> bool {
    true
}

fn default_true() -> bool {
    true
}
