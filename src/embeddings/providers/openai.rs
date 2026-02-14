// Supports:
// - text-embedding-3-small (1536 dimensions)
// - text-embedding-3-large (3072 dimensions)
// - text-embedding-ada-002 (1536 dimensions, legacy)

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use std::time::Duration;

use crate::embeddings::types::{Embedder, EmbeddingConfig, EmbeddingError, EmbeddingResponse, EmbeddingResult};
use crate::embeddings::cache::CachedEmbedder;

const DEFAULT_OPENAI_API_URL: &str = "https://api.openai.com/v1/embeddings";
const DEFAULT_CACHE_SIZE: usize = 10000;

// OpenAI embedding provider (with built-in LRU cache)
struct OpenAIEmbedderInner {
    client: Client, // Reqwest client for making HTTP requests to the OpenAI API
    api_key: String, // OpenAI API key for authentication
    model: String, // OpenAI model name, e.g. "text-embedding-3-small"
    base_url: String, // Base URL for the OpenAI API, defaulting to https://api.openai.com/v1/embeddings
}

pub struct OpenAIEmbedder {
    cached: CachedEmbedder<OpenAIEmbedderInner>,
}

impl OpenAIEmbedder {
    // Create a new OpenAI embedder with automatic caching (10K embeddings)
    pub fn new(config: &EmbeddingConfig) -> EmbeddingResult<Self> {
        let inner = OpenAIEmbedderInner::new(config)?;
        Ok(Self {
            cached: CachedEmbedder::new(inner, DEFAULT_CACHE_SIZE),
        })
    }

    // Create with custom cache size
    pub fn with_cache_size(config: &EmbeddingConfig, cache_size: usize) -> EmbeddingResult<Self> {
        let inner = OpenAIEmbedderInner::new(config)?;
        Ok(Self {
            cached: CachedEmbedder::new(inner, cache_size),
        })
    }
}

impl OpenAIEmbedderInner {
    fn new(config: &EmbeddingConfig) -> EmbeddingResult<Self> {
        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .ok_or_else(|| {
                EmbeddingError::ConfigError(
                    "OpenAI API key not provided in config or OPENAI_API_KEY env var".to_string(),
                )
            })?;

        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| DEFAULT_OPENAI_API_URL.to_string());

        let client = if let Some(timeout_secs) = config.timeout {
            reqwest::Client::builder()
                .timeout(Duration::from_secs(timeout_secs))
                .build()
                .map_err(|e| EmbeddingError::RequestFailed(e.to_string()))?
        } else {
            Client::new()
        };

        Ok(Self {
            client,
            api_key,
            model: config.model.clone(),
            base_url,
        })
    }

    // Get dimensions for known OpenAI models
    fn get_dimensions(&self) -> Option<usize> {
        match self.model.as_str() {
            "text-embedding-3-small" => Some(1536),
            "text-embedding-3-large" => Some(3072),
            "text-embedding-ada-002" => Some(1536),
            _ => None,
        }
    }
}

#[async_trait]
impl Embedder for OpenAIEmbedderInner {
    async fn embed(&self, text: &str) -> EmbeddingResult<EmbeddingResponse> {
        let request = OpenAIEmbeddingRequest {
            model: self.model.clone(),
            input: text.to_string(),
            encoding_format: Some("float".to_string()),
        };

        let response = self
            .client
            .post(&self.base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| EmbeddingError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            return Err(match status.as_u16() {
                401 => EmbeddingError::AuthenticationFailed(error_text),
                429 => EmbeddingError::RateLimitExceeded,
                _ => EmbeddingError::ApiError(format!("{}: {}", status, error_text)),
            });
        }

        let api_response: OpenAIEmbeddingResponse = response
            .json()
            .await
            .map_err(|e| EmbeddingError::InvalidResponse(e.to_string()))?;

        let first_embedding = api_response
            .data
            .first()
            .ok_or_else(|| EmbeddingError::InvalidResponse("No embeddings in response".to_string()))?;

        Ok(EmbeddingResponse {
            embedding: first_embedding.embedding.clone(),
            tokens: Some(api_response.usage.total_tokens),
            model: api_response.model,
        })
    }

    fn provider_name(&self) -> &str {
        "openai"
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn dimensions(&self) -> Option<usize> {
        self.get_dimensions()
    }
}

// Delegate Embedder trait to the cached inner embedder
#[async_trait]
impl Embedder for OpenAIEmbedder {
    async fn embed(&self, text: &str) -> EmbeddingResult<EmbeddingResponse> {
        self.cached.embed(text).await
    }

    fn provider_name(&self) -> &str {
        self.cached.provider_name()
    }

    fn model_name(&self) -> &str {
        self.cached.model_name()
    }

    fn dimensions(&self) -> Option<usize> {
        self.cached.dimensions()
    }
}

// OpenAI API types

#[derive(Debug, Serialize)]
struct OpenAIEmbeddingRequest {
    model: String,
    input: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    encoding_format: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIEmbeddingResponse {
    data: Vec<EmbeddingData>,
    model: String,
    usage: Usage,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
    index: usize,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: u32,
    total_tokens: u32,
}
