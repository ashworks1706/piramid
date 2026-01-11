//! Embedding providers module
//! 
//! This module provides a unified interface for different embedding providers,
//! allowing users to generate embeddings from text without needing to handle
//! the embeddings externally.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub mod openai;
pub mod ollama;
pub mod providers;

pub use providers::EmbeddingProvider;

/// Result type for embedding operations
pub type EmbeddingResult<T> = Result<T, EmbeddingError>;

/// Errors that can occur during embedding operations
#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
}

/// Configuration for embedding providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Provider type (openai, ollama, etc.)
    pub provider: String,
    
    /// Model name
    pub model: String,
    
    /// API key (for providers that require it)
    pub api_key: Option<String>,
    
    /// Base URL (for self-hosted or custom endpoints)
    pub base_url: Option<String>,
    
    /// Additional provider-specific options
    #[serde(default)]
    pub options: serde_json::Value,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            model: "text-embedding-3-small".to_string(),
            api_key: None,
            base_url: None,
            options: serde_json::json!({}),
        }
    }
}

/// Response from an embedding request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    /// The embedding vector
    pub embedding: Vec<f32>,
    
    /// Number of tokens used (if reported by provider)
    pub tokens: Option<u32>,
    
    /// Model that generated the embedding
    pub model: String,
}

/// Trait for embedding providers
#[async_trait]
pub trait Embedder: Send + Sync {
    /// Generate an embedding for a single text
    async fn embed(&self, text: &str) -> EmbeddingResult<EmbeddingResponse>;

    /// Generate embeddings for multiple texts in a batch
    async fn embed_batch(&self, texts: &[String]) -> EmbeddingResult<Vec<EmbeddingResponse>>;

    /// Get the provider name
    fn provider_name(&self) -> &str;

    /// Get the model name
    fn model_name(&self) -> &str;

    /// Get the expected dimension of embeddings (if known)
    fn dimensions(&self) -> Option<usize>;
}
