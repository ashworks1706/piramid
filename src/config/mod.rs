// Configuration module

mod app;
mod cache;
mod collection;
mod execution;
mod limits;
mod memory;
mod parallelism;
mod quantization;
mod search;
mod search_mode;
mod storage;
mod wal;
pub use crate::embeddings::EmbeddingConfig;
pub mod loader;

pub use app::AppConfig;
pub use cache::CacheConfig;
pub use collection::CollectionConfig;
pub use execution::ExecutionMode;
pub use limits::LimitsConfig;
pub use memory::MemoryConfig;
pub use parallelism::{ParallelismConfig, ParallelismMode};
pub use quantization::{QuantizationConfig, QuantizationLevel};
pub use search::SearchConfig;
pub use search_mode::{RangeSearchParams, SearchMode};
pub use storage::StorageConfig;
pub use wal::WalConfig;
