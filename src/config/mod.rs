// Configuration module for Piramid
// Centralized configuration management for all components

mod execution;
mod storage;
mod search;
mod quantization;
mod parallelism;
mod memory;
mod cache;
mod limits;
mod wal;
mod collection;
mod search_mode;
mod app;
pub use crate::embeddings::EmbeddingConfig;
pub mod loader;

pub use execution::ExecutionMode;
pub use storage::StorageConfig;
pub use search::SearchConfig;
pub use quantization::{QuantizationConfig, QuantizationLevel};
pub use parallelism::{ParallelismConfig, ParallelismMode};
pub use memory::MemoryConfig;
pub use cache::CacheConfig;
pub use limits::LimitsConfig;
pub use wal::WalConfig;
pub use collection::CollectionConfig;
pub use search_mode::{SearchMode, RangeSearchParams};
pub use app::AppConfig;
