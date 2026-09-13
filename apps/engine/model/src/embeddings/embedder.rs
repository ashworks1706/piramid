//! The Embedder contract and what a provider returns.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use piramid_core::error::embedding::EmbeddingError;

/// What a provider call returns.
pub type EmbeddingResult<T> = Result<T, EmbeddingError>;

/// One embedded text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    /// The vector the provider produced.
    pub embedding: Vec<f32>,

    /// Token count, when the provider reports one.
    pub tokens: Option<u32>,

    /// Model that produced the vector, as the provider names it.
    pub model: String,
}

/// A provider that turns text into a vector.
#[async_trait]
pub trait Embedder: Send + Sync {
    /// Embed one text.
    async fn embed(&self, text: &str) -> EmbeddingResult<EmbeddingResponse>;

    /// Short provider name, such as openai or ollama.
    fn provider_name(&self) -> &'static str;

    /// Model identifier requests are sent with.
    fn model_name(&self) -> &str;
}
