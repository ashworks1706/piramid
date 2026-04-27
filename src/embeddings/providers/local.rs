// Local embedding provider implementation 
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use std::time::Duration;

use crate::embeddings::types::{Embedder, EmbeddingConfig, EmbeddingError, EmbeddingResponse, EmbeddingResult};
use crate::embeddings::cache::CachedEmbedder;

/// Expects a POST to base_url with a JSON body containing model and input.
struct LocalEmbedderInner {
    client: Client,
    base_url: String,
    model: String,
}

pub struct LocalEmbedder {
    cached: CachedEmbedder<LocalEmbedderInner>,
}

impl LocalEmbedder {
    pub fn new(config: &EmbeddingConfig) -> EmbeddingResult<Self> {
        let inner = LocalEmbedderInner::new(config)?;
        Ok(Self {
            cached: CachedEmbedder::new(inner, 10_000),
        })
    }
}

impl LocalEmbedderInner {
    fn new(config: &EmbeddingConfig) -> EmbeddingResult<Self> {
        let base_url = config
            .base_url
            .clone()
            .ok_or_else(|| EmbeddingError::ConfigError("LOCAL provider requires base_url".into()))?;

        let client = if let Some(timeout_secs) = config.timeout {
            Client::builder()
                .timeout(Duration::from_secs(timeout_secs))
                .build()
                .map_err(|e| EmbeddingError::RequestFailed(e.to_string()))?
        } else {
            Client::new()
        };

        Ok(Self {
            client,
            model: config.model.clone(),
            base_url,
        })
    }
}

#[derive(Debug, Serialize)]
struct LocalEmbeddingRequest {
    model: String,
    input: String,
}

#[derive(Debug, Deserialize)]
struct LocalEmbeddingResponse {
    data: Vec<LocalEmbeddingData>,
    model: String,
    #[serde(default)]
    usage: Option<LocalUsage>,
}

#[derive(Debug, Deserialize)]
struct LocalEmbeddingData {
    embedding: Vec<f32>,
    #[serde(default)]
    #[allow(dead_code)] 
    index: Option<usize>,
}

#[derive(Debug, Deserialize, Default)]
struct LocalUsage {
    #[serde(default)]
    #[allow(dead_code)] 
    prompt_tokens: u32,
    #[serde(default)]
    total_tokens: u32,
}

#[async_trait]
impl Embedder for LocalEmbedderInner {
    async fn embed(&self, text: &str) -> EmbeddingResult<EmbeddingResponse> {

        // Construct the request body
        let req = LocalEmbeddingRequest {
            model: self.model.clone(),
            input: text.to_string(),
        };
        // Send the POST request to the local embedding endpoint
        let response = self
            .client
            .post(&self.base_url)
            .json(&req)
            .send()
            .await
            .map_err(|e| EmbeddingError::RequestFailed(e.to_string()))?;
        // Check for HTTP errors
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(EmbeddingError::ApiError(format!("{}: {}", status, error_text)));
        }
        // Parse the JSON response
        let api_resp: LocalEmbeddingResponse = response
            .json()
            .await
            .map_err(|e| EmbeddingError::InvalidResponse(e.to_string()))?;

        // Extract the embedding and token usage from the response
        let first = api_resp
            .data
            .first()
            .ok_or_else(|| EmbeddingError::InvalidResponse("No embeddings in response".into()))?;

        // Construct the EmbeddingResponse
        let tokens = api_resp.usage.as_ref().map(|u| u.total_tokens);

        // Return the embedding response
        Ok(EmbeddingResponse {
            embedding: first.embedding.clone(),
            tokens,
            model: api_resp.model,
        })
    }

    fn provider_name(&self) -> &str {
        "local"
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn dimensions(&self) -> Option<usize> {
        None
    }
}

#[async_trait]
impl Embedder for LocalEmbedder {
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
