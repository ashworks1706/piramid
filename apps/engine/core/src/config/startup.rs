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

    pub logging: LoggingConfig,
    pub telemetry: TelemetryConfig,
    pub hardware: HardwareConfig,
    pub disk: DiskConfig,
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
    /// Resolved worker-thread count. Zero leaves the choice to rayon.
    pub fn num_threads(&self) -> usize {
        self.threads.unwrap_or_else(num_cpus::get)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.bind.parse::<std::net::SocketAddr>().is_err() {
            return Err(format!(
                "startup.bind: '{}' is not an address:port",
                self.bind
            ));
        }
        if self.threads == Some(0) {
            return Err("startup.threads: must be > 0, or null for one per core".into());
        }
        if let Some(embedding) = &self.embedding {
            embedding.validate()?;
        }
        self.http.validate()?;
        self.hardware.gpu.validate()?;
        self.hardware.vram.validate()?;
        Ok(())
    }
}
