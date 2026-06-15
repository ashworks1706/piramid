// Configuration module

mod app;
mod cache;
mod collection;
mod execution;
mod hardware;
mod limits;
mod logging;
mod memory;
mod parallelism;
mod quantization;
mod search;
mod search_mode;
mod storage;
mod tuning;
mod wal;
pub use crate::embeddings::EmbeddingConfig;
pub mod loader;

pub use app::AppConfig;
pub use cache::CacheConfig;
pub use collection::CollectionConfig;
pub use execution::ExecutionMode;
pub use hardware::{HardwareConfig, HardwareProfile};
pub use limits::LimitsConfig;
pub use logging::{LogLevel, LoggingConfig};
pub use memory::MemoryConfig;
pub use parallelism::{ParallelismConfig, ParallelismMode};
pub use quantization::{QuantizationConfig, QuantizationLevel, QuantizationStage};
pub use search::SearchConfig;
pub use search_mode::{RangeSearchParams, SearchMode};
pub use storage::StorageConfig;
pub use tuning::{AdaptiveTuningConfig, QueryBudgetConfig};
pub use wal::WalConfig;
