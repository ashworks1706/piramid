//! Configuration, resolved from defaults, an optional file, then the environment.
//!
//! The file has two blocks: [StartupConfig] is fixed at boot, [RuntimeConfig] is re-read on
//! reload. config.example.yaml carries the whole surface.

mod cache;
mod collection;
mod console;
mod disk;
mod embedding;
mod file;
mod hardware;
mod http;
mod index;
mod index_params;
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

pub use cache::{
    CacheConfig, EmbeddingCacheConfig, EvictionPolicy, MetadataCacheConfig, VectorCacheConfig,
};
pub use collection::CollectionConfig;
pub use console::ConsoleConfig;
pub use disk::DiskConfig;
pub use embedding::{
    EmbeddingConfig, EmbeddingProvider, PiramidEmbeddingOptions, DEFAULT_OLLAMA_BASE_URL,
    DEFAULT_OPENAI_BASE_URL,
};
pub use file::Config;
pub use hardware::{GpuConfig, HardwareConfig, HardwareProfile, VramSplit};
pub use http::{ApiKey, AuthConfig, HttpConfig, RateLimitConfig, API_KEY_ENV};
pub use index::{AutoIndexConfig, IndexConfig, IndexKind};
pub use index_params::{FlatConfig, HnswConfig, IvfConfig};
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
