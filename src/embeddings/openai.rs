//! OpenAI embedding provider
//! 
//! Supports:
//! - text-embedding-3-small (1536 dimensions)
//! - text-embedding-3-large (3072 dimensions)
//! - text-embedding-ada-002 (1536 dimensions, legacy)

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use reqwest::Client;

use super::{Embedder, EmbeddingConfig, EmbeddingError, EmbeddingResponse, EmbeddingResult};

const DEFAULT_OPENAI_API_URL: &str = "https://api.openai.com/v1/embeddings";

/// OpenAI embedding provider
pub struct OpenAIEmbedder {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl OpenAIEmbedder {
    /// Create a new OpenAI embedder
    pub fn new(config: &EmbeddingConfig) -> EmbeddingResult<Self> {
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

        Ok(Self {
            client: Client::new(),
            api_key,
            model: config.model.clone(),
            base_url,
        })
    }

    /// Get dimensions for known OpenAI models
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
impl Embedder for OpenAIEmbedder {
    async fn embed(&self, text: &str) -> EmbeddingResult<EmbeddingResponse> {
        let request = OpenAIEmbeddingRequest {
            model: self.model.clone(),
            input: EmbeddingInput::Single(text.to_string()),
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

    async fn embed_batch(&self, texts: &[String]) -> EmbeddingResult<Vec<EmbeddingResponse>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let request = OpenAIEmbeddingRequest {
            model: self.model.clone(),
            input: EmbeddingInput::Batch(texts.to_vec()),
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

        let data_len = api_response.data.len();
        let total_tokens = api_response.usage.total_tokens;
        let model = api_response.model.clone();

        let results = api_response
            .data
            .into_iter()
            .map(|embedding_data| EmbeddingResponse {
                embedding: embedding_data.embedding,
                tokens: Some(total_tokens / data_len as u32),
                model: model.clone(),
            })
            .collect();

        Ok(results)
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

// OpenAI API types

#[derive(Debug, Serialize)]
struct OpenAIEmbeddingRequest {
    model: String,
    input: EmbeddingInput,
    #[serde(skip_serializing_if = "Option::is_none")]
    encoding_format: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum EmbeddingInput {
    Single(String),
    Batch(Vec<String>),
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
    index: usize,
}

#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: u32,
    total_tokens: u32,
}
