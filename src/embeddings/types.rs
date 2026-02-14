// Types for embedding system

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub use crate::error::embedding::EmbeddingError;

pub type EmbeddingResult<T> = Result<T, EmbeddingError>;

// Configuration for embedding providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    // Provider type (openai, ollama, etc.)
    pub provider: String,
    
    // Model name
    pub model: String,
    
    // API key (for providers that require it)
    pub api_key: Option<String>,
    
    // Base URL (for self-hosted or custom endpoints)
    pub base_url: Option<String>,
    
    // Additional provider-specific options
    #[serde(default)]
    pub options: serde_json::Value,

    // timeout for embedding requests (in seconds)
    #[serde(default)]
    pub timeout: Option<u64>,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            model: "text-embedding-3-small".to_string(),
            api_key: None,
            base_url: None,
            options: serde_json::json!({}),
            timeout: None,
        }
    }
}

// Response from an embedding request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    // The embedding vector
    pub embedding: Vec<f32>,
    
    // Number of tokens used (if reported by provider)
    pub tokens: Option<u32>,
    
    // Model that generated the embedding
    pub model: String,
}

// Trait for embedding providers
#[async_trait]
pub trait Embedder: Send + Sync {
    // Generate an embedding for a single text
    async fn embed(&self, text: &str) -> EmbeddingResult<EmbeddingResponse>;

    // Generate embeddings for multiple texts in a batch
    // Get the provider name
    fn provider_name(&self) -> &str;

    // Get the model name
    fn model_name(&self) -> &str;

    // Get the expected dimension of embeddings (if known)
    fn dimensions(&self) -> Option<usize>;
}
