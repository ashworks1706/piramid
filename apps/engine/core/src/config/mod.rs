//! Configuration, resolved from defaults, an optional file, then the environment.

mod collection;
mod console;
mod disk;
mod embedding;
mod file;
mod hardware;
mod http;
mod inference;
mod limits;
pub mod loader;
mod logging;
mod memory;
mod quantization;
mod runtime;
mod search;
mod startup;
mod telemetry;
mod wal;

pub use collection::CollectionConfig;
pub use console::ConsoleConfig;
pub use disk::DiskConfig;
pub use embedding::{
    EmbeddingCacheConfig, EmbeddingConfig, EmbeddingProvider, PiramidEmbeddingOptions,
    DEFAULT_OLLAMA_BASE_URL, DEFAULT_OPENAI_BASE_URL,
};
pub use file::Config;
pub use hardware::{GpuConfig, HardwareConfig, HardwareProfile, VramSplit};
pub use http::{ApiKey, AuthConfig, HttpConfig, RateLimitConfig, API_KEY_ENV};
pub use inference::{
    BatchingConfig, DeadlineMiss, DeviceSelection, DocumentKvConfig, DocumentKvStorage, Dtype,
    FusionConfig, InferenceConfig, KvCacheConfig, Preemption, RetrievalPointKind, SamplingConfig,
};
pub use limits::LimitsConfig;
pub use logging::{LogLevel, LoggingConfig};
pub use memory::MemoryConfig;
pub use quantization::{QuantizationConfig, QuantizationLevel, QuantizationStage};
pub use runtime::RuntimeConfig;
pub use search::SearchConfig;
pub use startup::StartupConfig;
pub use telemetry::{OtlpConfig, TelemetryConfig};
pub use wal::WalConfig;

/// Shared serde default target for fields that default to on.
pub(crate) fn default_true() -> bool {
    true
}
