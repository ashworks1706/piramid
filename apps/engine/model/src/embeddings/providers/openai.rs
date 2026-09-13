//! The OpenAI embeddings wire format; works with any server speaking the same protocol.

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
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
    options: serde_json::Map<String, serde_json::Value>,
}

impl OpenAIEmbedder {
    /// The JSON body for one text: the configured options, then the fields the provider sets.
    fn request_body(&self, text: &str) -> serde_json::Map<String, serde_json::Value> {
        let mut request = self.options.clone();
        request.insert("model".into(), self.model.clone().into());
        request.insert("input".into(), text.into());
        request.insert("encoding_format".into(), "float".into());
        request
    }

    /// A client for the configured model. base_url is the full endpoint URL; unset is the OpenAI
    /// endpoint.
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
            options: super::options::request_options(&config.options)?,
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
fn status_error(status: reqwest::StatusCode, body: String) -> EmbeddingError {
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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "a failed assertion is the point of a test"
)]
mod tests {
    use super::*;

    #[test]
    fn options_are_merged_into_the_request_body() {
        let config: EmbeddingConfig = serde_json::from_value(serde_json::json!({
            "provider": "openai",
            "model": "text-embedding-3-small",
            "options": { "dimensions": 256, "user": "docs" }
        }))
        .unwrap();
        let body = OpenAIEmbedder::new(&config).unwrap().request_body("hello");
        assert_eq!(body["dimensions"], 256);
        assert_eq!(body["user"], "docs");
        assert_eq!(body["model"], "text-embedding-3-small");
        assert_eq!(body["input"], "hello");
    }

    #[test]
    fn refused_input_and_unknown_models_are_not_retried() {
        let error =
            |code: u16| status_error(reqwest::StatusCode::from_u16(code).unwrap(), "no".into());
        for code in [400, 422] {
            assert!(
                matches!(error(code), EmbeddingError::InvalidInput(_)),
                "{code}"
            );
            assert!(!error(code).is_recoverable(), "{code}");
        }
        assert!(matches!(error(404), EmbeddingError::InvalidModel(_)));
        assert!(!error(404).is_recoverable());
        assert!(matches!(
            error(401),
            EmbeddingError::AuthenticationFailed(_)
        ));
        assert!(matches!(error(429), EmbeddingError::RateLimitExceeded));
        assert!(matches!(error(503), EmbeddingError::ApiError(_)));
        assert!(error(503).is_recoverable());
    }
}
