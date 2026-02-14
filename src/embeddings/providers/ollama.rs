// Supports local embedding models via Ollama:
// - nomic-embed-text (768 dimensions)
// - mxbai-embed-large (1024 dimensions)
// - all-minilm (384 dimensions)

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use reqwest::Client;

use crate::embeddings::types::{Embedder, EmbeddingConfig, EmbeddingError, EmbeddingResponse, EmbeddingResult};
use crate::embeddings::cache::CachedEmbedder;

const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";
const DEFAULT_CACHE_SIZE: usize = 10000;

// Ollama embedding provider (with built-in LRU cache)
struct OllamaEmbedderInner {
    client: Client,
    model: String,
    base_url: String,
}

pub struct OllamaEmbedder {
    cached: CachedEmbedder<OllamaEmbedderInner>,
}

impl OllamaEmbedder {
    // Create a new Ollama embedder with automatic caching (10K embeddings)
    pub fn new(config: &EmbeddingConfig) -> EmbeddingResult<Self> {
        let inner = OllamaEmbedderInner::new(config)?;
        Ok(Self {
            cached: CachedEmbedder::new(inner, DEFAULT_CACHE_SIZE),
        })
    }

    // Create with custom cache size
    pub fn with_cache_size(config: &EmbeddingConfig, cache_size: usize) -> EmbeddingResult<Self> {
        let inner = OllamaEmbedderInner::new(config)?;
        Ok(Self {
            cached: CachedEmbedder::new(inner, cache_size),
        })
    }
}

impl OllamaEmbedderInner {
    fn new(config: &EmbeddingConfig) -> EmbeddingResult<Self> {
        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| DEFAULT_OLLAMA_URL.to_string());

        Ok(Self {
            client: Client::new(),
            model: config.model.clone(),
            base_url,
        })
    }

    // Get dimensions for known Ollama models
    fn get_dimensions(&self) -> Option<usize> {
        match self.model.as_str() {
            "nomic-embed-text" => Some(768),
            "mxbai-embed-large" => Some(1024),
            "all-minilm" => Some(384),
            _ => None,
        }
    }
}

#[async_trait]
impl Embedder for OllamaEmbedderInner {
    async fn embed(&self, text: &str) -> EmbeddingResult<EmbeddingResponse> {

        // Construct the request payload for the Ollama API. The request includes the model name and the input text (prompt) that we want to embed. We will send this request to the /api/embeddings endpoint of the Ollama server, which will return the embedding vector for the input text.
        let request = OllamaEmbeddingRequest {
            model: self.model.clone(),
            prompt: text.to_string(),
        };

        let url = format!("{}/api/embeddings", self.base_url);

        // Send the request to the Ollama API and handle the response. We use the reqwest client to send a POST request with the JSON payload. If the request fails (e.g., network error), we return an EmbeddingError. If the response status is not successful, we read the error message from the response and return it as an API error. If the response is successful, we parse the JSON response into our OllamaEmbeddingResponse struct, which contains the embedding vector.
        let response = self
            .client
            .post(&url)
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

            return Err(EmbeddingError::ApiError(format!("{}: {}", status, error_text)));
        }

        // Parse the successful response from the Ollama API. The response is expected to be a JSON object that contains the embedding vector. We deserialize this JSON into our OllamaEmbeddingResponse struct. If deserialization fails, we return an InvalidResponse error.
        let api_response: OllamaEmbeddingResponse = response
            .json()
            .await
            .map_err(|e| EmbeddingError::InvalidResponse(e.to_string()))?;


        Ok(EmbeddingResponse {
            embedding: api_response.embedding,
            tokens: None, // Ollama doesn't report token usage
            model: self.model.clone(),
        })
    }

    fn provider_name(&self) -> &str {
        "ollama"
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
impl Embedder for OllamaEmbedder {
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

// Ollama API types

#[derive(Debug, Serialize)]
struct OllamaEmbeddingRequest {
    model: String,
    prompt: String,
}

#[derive(Debug, Deserialize)]
struct OllamaEmbeddingResponse {
    embedding: Vec<f32>,
}
