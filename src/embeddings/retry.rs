// Retry wrapper for embedders with exponential backoff
// This module provides a RetryEmbedder that wraps any Embedder implementation and adds retry logic with exponential backoff. If an embedding request fails due to a transient error (e.g., network issue, rate limit), the RetryEmbedder will automatically retry the request up to a specified number of times, with increasing delays between attempts. This helps improve the robustness of embedding requests by handling temporary failures gracefully.
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use crate::embeddings::{Embedder, EmbeddingResponse, EmbeddingResult, EmbeddingError};

// RetryEmbedder wraps another Embedder and adds retry logic with exponential backoff
pub struct RetryEmbedder {
    inner: Arc<dyn Embedder>,
    max_retries: u32,
    initial_delay_ms: u64,
    max_delay_ms: u64,
}

// Configuration options for the RetryEmbedder
#[derive(Clone, Debug)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
}

// Default retry configuration: 3 retries, starting at 1 second, doubling each time, up to 30 seconds
impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
        }
    }
}

// Implementation of the RetryEmbedder
impl RetryEmbedder {
    pub fn new(embedder: Arc<dyn Embedder>) -> Self {
        Self::with_options(embedder, RetryConfig::default())
    }
    
    pub fn with_options(embedder: Arc<dyn Embedder>, options: RetryConfig) -> Self {
        Self {
            inner: embedder,
            max_retries: options.max_retries,
            initial_delay_ms: options.initial_delay_ms,
            max_delay_ms: options.max_delay_ms,
        }
    }
}

// Implement the Embedder trait for RetryEmbedder. The embed method will attempt to call the inner embedder's embed method, and if it fails with a retryable error, it will wait for a certain amount of time (starting with initial_delay_ms and doubling each time) before retrying, up to max_retries times. If all attempts fail, it will return the last error.
#[async_trait]
impl Embedder for RetryEmbedder {
    async fn embed(&self, text: &str) -> EmbeddingResult<EmbeddingResponse> {
        // We will keep track of the number of attempts and the current delay. We will loop until we either get a successful response or exhaust our retries. On each failure, we check if the error is retryable. If it is not retryable, we return the error immediately. If it is retryable, we log the failure and wait for the specified delay before trying again. The delay increases exponentially with each attempt, up to a maximum limit.
        let mut attempts = 0;
        let mut delay_ms = self.initial_delay_ms;
        
        // Loop to attempt embedding with retries
        loop {
            match self.inner.embed(text).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    attempts += 1;
                    
                    if !is_retryable_error(&e) || attempts > self.max_retries {
                        return Err(e);
                    }
                    
                    eprintln!(
                        "Embedding request failed (attempt {}/{}): {}. Retrying in {}ms...",
                        attempts, self.max_retries + 1, e, delay_ms
                    );
                    
                    sleep(Duration::from_millis(delay_ms)).await;
                    delay_ms = (delay_ms * 2).min(self.max_delay_ms);
                }
            }
        }
    }

    fn provider_name(&self) -> &str {
        self.inner.provider_name()
    }

    fn model_name(&self) -> &str {
        self.inner.model_name()
    }

    fn dimensions(&self) -> Option<usize> {
        self.inner.dimensions()
    }
}

// Determine if an error is retryable
fn is_retryable_error(error: &EmbeddingError) -> bool {
    // Use the built-in is_recoverable method
    error.is_recoverable()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    struct MockEmbedder {
        calls: Arc<AtomicU32>,
        fail_count: u32,
    }

    impl MockEmbedder {
        fn new(fail_count: u32) -> Self {
            Self {
                calls: Arc::new(AtomicU32::new(0)),
                fail_count,
            }
        }
    }
    #[async_trait]
    impl Embedder for MockEmbedder {
        async fn embed(&self, _text: &str) -> EmbeddingResult<EmbeddingResponse> {
            let call_num = self.calls.fetch_add(1, Ordering::SeqCst);
            
            if call_num < self.fail_count {
                return Err(EmbeddingError::RequestFailed("Connection failed".to_string()));
            }
            
            Ok(EmbeddingResponse {
                embedding: vec![1.0, 2.0, 3.0],
                tokens: Some(10),
                model: "test".to_string(),
            })
        }

        fn provider_name(&self) -> &str {
            "mock"
        }

        fn model_name(&self) -> &str {
            "test-model"
        }

        fn dimensions(&self) -> Option<usize> {
            Some(3)
        }
    }

    #[tokio::test]
    async fn test_retry_success_after_failures() {
        let mock = MockEmbedder::new(2); // Fail first 2 attempts
        let calls_counter = mock.calls.clone();
        let retry = RetryEmbedder::with_options(
            Arc::new(mock),
            RetryConfig {
                max_retries: 3,
                initial_delay_ms: 10,
                max_delay_ms: 1000,
            },
        );
        
        let result = retry.embed("test").await;
        assert!(result.is_ok());
        assert_eq!(calls_counter.load(Ordering::SeqCst), 3); // 2 failures + 1 success
    }

    #[tokio::test]
    async fn test_retry_exhaustion() {
        let mock = MockEmbedder::new(10); // Always fail
        let retry = RetryEmbedder::with_options(
            Arc::new(mock),
            RetryConfig {
                max_retries: 2,
                initial_delay_ms: 10,
                max_delay_ms: 1000,
            },
        );
        
        let result = retry.embed("test").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_retryable_errors() {
        assert!(is_retryable_error(&EmbeddingError::RequestFailed("test".to_string())));
        assert!(is_retryable_error(&EmbeddingError::Timeout("test".to_string())));
        assert!(is_retryable_error(&EmbeddingError::RateLimitExceeded));
        assert!(is_retryable_error(&EmbeddingError::ProviderUnavailable("test".to_string())));
        assert!(!is_retryable_error(&EmbeddingError::AuthenticationFailed("test".to_string())));
        assert!(!is_retryable_error(&EmbeddingError::ConfigError("test".to_string())));
    }
}
