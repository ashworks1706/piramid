//! The OpenAI embeddings wire format; works with any server speaking the same protocol.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::embeddings::embedder::{Embedder, EmbeddingResponse, EmbeddingResult};
use piramid_core::config::EmbeddingConfig;
use piramid_core::error::embedding::EmbeddingError;

const DEFAULT_OPENAI_API_URL: &str = "https://api.openai.com/v1/embeddings";

/// Embeds text through an endpoint speaking the OpenAI embeddings format.
pub struct OpenAIEmbedder {
    client: Client,
    api_key: Option<String>,
    model: String,
    base_url: String,
}

impl OpenAIEmbedder {
    /// A client for the configured model. base_url is the full endpoint URL; unset is OpenAI's.
    // The key reaches config from OPENAI_API_KEY in the loader.
    pub fn new(config: &EmbeddingConfig) -> EmbeddingResult<Self> {
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
            api_key: config.api_key.clone(),
            model: config.model.clone(),
            base_url,
        })
    }
}

#[async_trait]
impl Embedder for OpenAIEmbedder {
    async fn embed(&self, text: &str) -> EmbeddingResult<EmbeddingResponse> {
        let request = OpenAIEmbeddingRequest {
            model: self.model.clone(),
            input: text.to_string(),
            encoding_format: Some("float".to_string()),
        };

        let mut post = self
            .client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .json(&request);
        if let Some(key) = &self.api_key {
            post = post.header("Authorization", format!("Bearer {key}"));
        }

        let response = post
            .send()
            .await
            .map_err(|e| EmbeddingError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|error| format!("<body unreadable: {error}>"));

            return Err(match status.as_u16() {
                401 => EmbeddingError::AuthenticationFailed(error_text),
                429 => EmbeddingError::RateLimitExceeded,
                _ => EmbeddingError::ApiError(format!("{status}: {error_text}")),
            });
        }

        let api_response: OpenAIEmbeddingResponse = response
            .json()
            .await
            .map_err(|e| EmbeddingError::InvalidResponse(e.to_string()))?;

        let first_embedding = api_response.data.first().ok_or_else(|| {
            EmbeddingError::InvalidResponse("No embeddings in response".to_string())
        })?;

        Ok(EmbeddingResponse {
            embedding: first_embedding.embedding.clone(),
            tokens: Some(api_response.usage.total_tokens),
            model: api_response.model,
        })
    }

    fn provider_name(&self) -> &'static str {
        "openai"
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn dimensions(&self) -> Option<usize> {
        match self.model.as_str() {
            "text-embedding-3-small" => Some(1536),
            "text-embedding-3-large" => Some(3072),
            _ => None,
        }
    }
}

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

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

#[derive(Debug, Deserialize)]
struct Usage {
    total_tokens: u32,
}
