#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]

use piramid_model::embeddings::{CachedEmbedder, Embedder, EmbeddingResponse, EmbeddingResult};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

struct MockEmbedder {
    call_count: Arc<AtomicUsize>,
}

#[async_trait::async_trait]
impl Embedder for MockEmbedder {
    async fn embed(&self, text: &str) -> EmbeddingResult<EmbeddingResponse> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        Ok(EmbeddingResponse {
            embedding: vec![text.len() as f32],
            tokens: Some(1),
            model: "mock".to_string(),
        })
    }

    fn provider_name(&self) -> &'static str {
        "mock"
    }

    fn model_name(&self) -> &str {
        "mock-model"
    }

    fn dimensions(&self) -> Option<usize> {
        Some(1)
    }
}

#[tokio::test]
async fn cache_hits_and_eviction() {
    let call_count = Arc::new(AtomicUsize::new(0));
    let mock = MockEmbedder {
        call_count: call_count.clone(),
    };
    let cached = CachedEmbedder::new(mock, std::num::NonZeroUsize::new(2).unwrap());

    cached.embed("hello").await.unwrap();
    assert_eq!(call_count.load(Ordering::SeqCst), 1);

    cached.embed("hello").await.unwrap();
    assert_eq!(call_count.load(Ordering::SeqCst), 1); // cache hit

    cached.embed("world").await.unwrap();
    assert_eq!(call_count.load(Ordering::SeqCst), 2);

    cached.embed("third").await.unwrap(); // evicts LRU
    assert_eq!(call_count.load(Ordering::SeqCst), 3);

    cached.embed("hello").await.unwrap(); // hello likely evicted, another call
    assert!(call_count.load(Ordering::SeqCst) >= 4);
}

// A hit returns the model the provider reported when the text was embedded, and no token count,
// since nothing was sent.
#[tokio::test]
async fn a_hit_reports_what_the_provider_said() {
    let call_count = Arc::new(AtomicUsize::new(0));
    let cached = CachedEmbedder::new(
        MockEmbedder {
            call_count: call_count.clone(),
        },
        std::num::NonZeroUsize::new(4).unwrap(),
    );

    let miss = cached.embed("hello").await.unwrap();
    let hit = cached.embed("hello").await.unwrap();
    assert_eq!(call_count.load(Ordering::SeqCst), 1);
    assert_eq!(miss.model, "mock");
    assert_eq!(hit.model, miss.model);
    assert_eq!(hit.tokens, None);
}
