//! Retry wrapper for embedding providers, with exponential backoff on recoverable errors.

use crate::embeddings::{Embedder, EmbeddingError, EmbeddingResponse, EmbeddingResult};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

/// An [Embedder] that retries recoverable failures with doubling delays.
pub struct RetryEmbedder {
    inner: Arc<dyn Embedder>,
    max_retries: u32,
    initial_delay_ms: u64,
    max_delay_ms: u64,
}

/// Retry count and backoff bounds for a [RetryEmbedder].
#[derive(Clone, Debug)]
pub struct RetryConfig {
    /// Attempts after the first before the error is returned.
    pub max_retries: u32,
    /// Delay before the first retry, in milliseconds.
    pub initial_delay_ms: u64,
    /// Ceiling the doubling delay stops at, in milliseconds.
    pub max_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
        }
    }
}

impl RetryEmbedder {
    /// Wrap embedder with the default retry settings.
    pub fn new(embedder: Arc<dyn Embedder>) -> Self {
        Self::with_options(embedder, RetryConfig::default())
    }

    /// Wrap embedder with the given retry settings.
    pub fn with_options(embedder: Arc<dyn Embedder>, options: RetryConfig) -> Self {
        Self {
            inner: embedder,
            max_retries: options.max_retries,
            initial_delay_ms: options.initial_delay_ms,
            max_delay_ms: options.max_delay_ms,
        }
    }
}

#[async_trait]
impl Embedder for RetryEmbedder {
    async fn embed(&self, text: &str) -> EmbeddingResult<EmbeddingResponse> {
        let mut attempts = 0;
        let mut delay_ms = self.initial_delay_ms;

        loop {
            match self.inner.embed(text).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    attempts += 1;

                    if !is_retryable_error(&e) || attempts > self.max_retries {
                        return Err(e);
                    }

                    tracing::warn!(
                        attempt = attempts,
                        max_retries = self.max_retries,
                        delay_ms,
                        error = %e,
                        "embedding_request_retrying"
                    );

                    sleep(Duration::from_millis(delay_ms)).await;
                    delay_ms = (delay_ms * 2).min(self.max_delay_ms);
                }
            }
        }
    }

    fn provider_name(&self) -> &'static str {
        self.inner.provider_name()
    }

    fn model_name(&self) -> &str {
        self.inner.model_name()
    }
}

fn is_retryable_error(error: &EmbeddingError) -> bool {
    error.is_recoverable()
}
