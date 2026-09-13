//! Embedding providers: turns text into vectors.

pub mod cache;
mod embedder;
mod manager;
pub mod providers;
pub mod retry;

pub use cache::CachedEmbedder;
pub use embedder::{Embedder, EmbeddingResponse, EmbeddingResult};
pub use manager::EmbeddingsManager;
pub use piramid_core::error::embedding::EmbeddingError;
pub use providers::create_embedder;
pub use retry::RetryEmbedder;
