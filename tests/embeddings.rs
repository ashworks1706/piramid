use piramid::embeddings::retry::RetryConfig;
use piramid::embeddings::{
    Embedder, EmbeddingError, EmbeddingProvider, EmbeddingResponse, EmbeddingResult, RetryEmbedder,
};
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

#[async_trait::async_trait]
impl Embedder for MockEmbedder {
    async fn embed(&self, _text: &str) -> EmbeddingResult<EmbeddingResponse> {
        let call_num = self.calls.fetch_add(1, Ordering::SeqCst);
        if call_num < self.fail_count {
            return Err(EmbeddingError::RequestFailed("fail".into()));
        }
        Ok(EmbeddingResponse {
            embedding: vec![1.0, 2.0, 3.0],
            tokens: Some(10),
            model: "test".into(),
        })
    }

    fn provider_name(&self) -> &str {
        "mock"
    }

    fn model_name(&self) -> &str {
        "mock-model"
    }

    fn dimensions(&self) -> Option<usize> {
        Some(3)
    }
}

#[tokio::test]
async fn retry_succeeds_after_failures() {
    let embedder = MockEmbedder::new(2);
    let calls = embedder.calls.clone();
    let retry = RetryEmbedder::with_options(
        Arc::new(embedder),
        RetryConfig {
            max_retries: 3,
            initial_delay_ms: 1,
            max_delay_ms: 10,
        },
    );

    let res = retry.embed("hello").await;
    assert!(res.is_ok());
    assert_eq!(calls.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn retry_exhausts_and_errors() {
    let embedder = MockEmbedder::new(5);
    let retry = RetryEmbedder::with_options(
        Arc::new(embedder),
        RetryConfig {
            max_retries: 2,
            initial_delay_ms: 1,
            max_delay_ms: 10,
        },
    );

    let res = retry.embed("hello").await;
    assert!(res.is_err());
}

#[test]
fn provider_from_str_roundtrip() {
    assert_eq!(
        "openai".parse::<EmbeddingProvider>(),
        Ok(EmbeddingProvider::OpenAI)
    );
    assert_eq!(
        "ollama".parse::<EmbeddingProvider>(),
        Ok(EmbeddingProvider::Ollama)
    );
    assert_eq!(
        "local".parse::<EmbeddingProvider>(),
        Ok(EmbeddingProvider::Local)
    );
    assert!("unknown".parse::<EmbeddingProvider>().is_err());
}
