// allowing users to generate embeddings from text without needing to handle the embeddings externally

pub mod cache;
pub mod providers;
pub mod retry;
mod types;

pub use crate::error::embedding::EmbeddingError;
pub use cache::{CacheStats, CachedEmbedder};
pub use providers::{create_embedder, EmbeddingProvider};
pub use retry::RetryEmbedder;
pub use types::{Embedder, EmbeddingConfig, EmbeddingResponse, EmbeddingResult};
