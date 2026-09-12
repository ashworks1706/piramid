//! An Embedder wrapper that caches by text, evicting least-recently-used entries.

use async_trait::async_trait;
use lru::LruCache;
use parking_lot::Mutex;
use std::num::NonZeroUsize;

use super::embedder::{Embedder, EmbeddingResponse, EmbeddingResult};

/// An embedding and the model the provider said produced it.
struct CachedEmbedding {
    embedding: Vec<f32>,
    model: String,
}

/// An [Embedder] that answers repeated text from a bounded least-recently-used cache.
pub struct CachedEmbedder<E: Embedder> {
    inner: E,
    cache: Mutex<LruCache<String, CachedEmbedding>>,
}

impl<E: Embedder> CachedEmbedder<E> {
    /// Wrap embedder with a cache holding up to capacity texts.
    pub fn new(embedder: E, capacity: NonZeroUsize) -> Self {
        Self {
            inner: embedder,
            cache: Mutex::new(LruCache::new(capacity)),
        }
    }
}

#[async_trait]
impl<E: Embedder> Embedder for CachedEmbedder<E> {
    async fn embed(&self, text: &str) -> EmbeddingResult<EmbeddingResponse> {
        {
            let mut cache = self.cache.lock();
            // A hit sends nothing to the provider, so it consumes no tokens.
            if let Some(hit) = cache.get(text) {
                return Ok(EmbeddingResponse {
                    embedding: hit.embedding.clone(),
                    tokens: None,
                    model: hit.model.clone(),
                });
            }
        }

        let response = self.inner.embed(text).await?;

        {
            let mut cache = self.cache.lock();
            cache.put(
                text.to_string(),
                CachedEmbedding {
                    embedding: response.embedding.clone(),
                    model: response.model.clone(),
                },
            );
        }

        Ok(response)
    }

    fn provider_name(&self) -> &'static str {
        self.inner.provider_name()
    }

    fn model_name(&self) -> &str {
        self.inner.model_name()
    }

    fn dimensions(&self) -> Option<usize> {
        self.inner.dimensions()
    }
}
