//! Settings applied once, when the process starts.
//!
//! Changing any of these needs a restart. The listener is bound, the thread pool built and the
//! tracing subscriber installed before the first request, and /config/reload refuses a file whose
//! startup block differs from the one the process booted with.

use serde::{Deserialize, Serialize};

use super::{
    DiskConfig, EmbeddingConfig, HardwareConfig, HttpConfig, LoggingConfig, TelemetryConfig,
};

/// Everything fixed at boot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct StartupConfig {
    /// Address to listen on.
    pub bind: String,

    /// Root directory for collection data.
    pub data_dir: String,

    /// Worker threads for parallel search and indexing. None is one per core.
    pub threads: Option<usize>,

    /// Log level, format and per-subsystem switches.
    pub logging: LoggingConfig,
    /// Trace export and span events.
    pub telemetry: TelemetryConfig,
    /// Hardware selection and memory budgets.
    pub hardware: HardwareConfig,
    /// Free-space checks on the data directory.
    pub disk: DiskConfig,
    /// Authentication, rate limiting and shutdown for the HTTP server.
    pub http: HttpConfig,

    /// Embedding provider, built once at boot. None disables server-side embedding.
    pub embedding: Option<EmbeddingConfig>,
}

impl Default for StartupConfig {
    fn default() -> Self {
        StartupConfig {
            bind: "127.0.0.1:6333".to_string(),
            data_dir: "./data".to_string(),
            threads: None,
            logging: LoggingConfig::default(),
            telemetry: TelemetryConfig::default(),
            hardware: HardwareConfig::default(),
            disk: DiskConfig::default(),
            http: HttpConfig::default(),
            embedding: None,
        }
    }
}

impl StartupConfig {
    /// Resolved worker-thread count: threads when set, otherwise one per CPU.
    pub fn num_threads(&self) -> usize {
        self.threads.unwrap_or_else(num_cpus::get)
    }

    /// Reject an unparseable bind address, zero threads, an OTLP block this build or logging cannot
    /// export, an unenforced memory budget, or an invalid embedding, GPU or HTTP setting.
    pub fn validate(&self) -> Result<(), String> {
        if self.bind.parse::<std::net::SocketAddr>().is_err() {
            return Err(format!(
                "startup.bind: '{}' is not an address:port",
                self.bind
            ));
        }
        if self.telemetry.otlp.is_some() {
            if !cfg!(feature = "otel") {
                return Err(
                    "startup.telemetry.otlp: this build lacks the otel feature, so spans cannot \
                     be exported"
                        .into(),
                );
            }
            if !self.logging.enabled {
                return Err(
                    "startup.telemetry.otlp: spans are exported through the tracing subscriber, \
                     which startup.logging.enabled: false does not install"
                        .into(),
                );
            }
        }
        if self.threads == Some(0) {
            return Err("startup.threads: must be > 0, or null for one per core".into());
        }
        if let Some(embedding) = &self.embedding {
            embedding.validate()?;
        }
        if self.hardware.memory_budget().is_some() {
            return Err(
                "startup.hardware: a host memory budget, from memory_budget_bytes or a memory-class \
                 profile, is not enforced yet"
                    .into(),
            );
        }
        self.http.validate()?;
        self.hardware.gpu.validate()?;
        self.hardware.vram.validate()?;
        Ok(())
    }
}
