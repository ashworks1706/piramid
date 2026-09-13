//! The OpenAI embeddings wire format; works with any server speaking the same protocol.

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

use crate::embeddings::embedder::{Embedder, EmbeddingResponse, EmbeddingResult};
use piramid_core::config::{EmbeddingConfig, DEFAULT_OPENAI_BASE_URL};
use piramid_core::error::embedding::EmbeddingError;

/// Embeds text through an endpoint speaking the OpenAI embeddings format.
pub struct OpenAIEmbedder {
    client: Client,
    api_key: Option<String>,
    model: String,
    base_url: String,
    options: serde_json::Map<String, serde_json::Value>,
}

impl OpenAIEmbedder {
    /// The JSON body for one text: the configured options, then the fields the provider sets.
    pub fn request_body(&self, text: &str) -> serde_json::Map<String, serde_json::Value> {
        let mut request = self.options.clone();
        request.insert("model".into(), self.model.clone().into());
        request.insert("input".into(), text.into());
        request.insert("encoding_format".into(), "float".into());
        request
    }

    /// A client for the configured model; unset base_url falls back to the OpenAI endpoint.
    pub fn new(config: &EmbeddingConfig) -> EmbeddingResult<Self> {
        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| DEFAULT_OPENAI_BASE_URL.to_string());

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
            options: config
                .request_options()
                .map_err(EmbeddingError::ConfigError)?,
        })
    }
}

#[async_trait]
impl Embedder for OpenAIEmbedder {
    async fn embed(&self, text: &str) -> EmbeddingResult<EmbeddingResponse> {
        let request = self.request_body(text);

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

            return Err(status_error(status, error_text));
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
}

/// The error for a response with a failure status and its body.
pub fn status_error(status: reqwest::StatusCode, body: String) -> EmbeddingError {
    match status.as_u16() {
        400 | 422 => EmbeddingError::InvalidInput(format!("{status}: {body}")),
        401 => EmbeddingError::AuthenticationFailed(body),
        404 => EmbeddingError::InvalidModel(format!("{status}: {body}")),
        429 => EmbeddingError::RateLimitExceeded,
        _ => EmbeddingError::ApiError(format!("{status}: {body}")),
    }
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
