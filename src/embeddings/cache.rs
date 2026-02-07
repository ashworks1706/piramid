// LRU cache wrapper for embeddings to reduce API costs
//
// Caches embeddings in memory to avoid redundant API calls.
// Uses LRU eviction when cache is full.

use async_trait::async_trait;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Mutex;

use super::{Embedder, EmbeddingResponse, EmbeddingResult};

pub struct CachedEmbedder<E: Embedder> {
    inner: E,
    cache: Mutex<LruCache<String, Vec<f32>>>,
}

impl<E: Embedder> CachedEmbedder<E> {
    pub fn new(embedder: E, capacity: usize) -> Self {
        let capacity = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(10000).unwrap());
        Self {
            inner: embedder,
            cache: Mutex::new(LruCache::new(capacity)),
        }
    }

    // Get cache statistics
    pub fn cache_stats(&self) -> CacheStats {
        let cache = self.cache.lock().unwrap();
        CacheStats {
            size: cache.len(),
            capacity: cache.cap().get(),
        }
    }

    // Clear the cache
    pub fn clear_cache(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
    }
}

#[async_trait]
impl<E: Embedder> Embedder for CachedEmbedder<E> {
    async fn embed(&self, text: &str) -> EmbeddingResult<EmbeddingResponse> {
        // Check cache first
        {
            let mut cache = self.cache.lock().unwrap();
            if let Some(embedding) = cache.get(text) {
                // Cache hit! Return immediately
                return Ok(EmbeddingResponse {
                    embedding: embedding.clone(),
                    tokens: None,  // We don't track tokens for cached results
                    model: self.inner.model_name().to_string(),
                });
            }
        }
        
        // Cache miss - call the underlying embedder
        let response = self.inner.embed(text).await?;
        
        // Store in cache
        {
            let mut cache = self.cache.lock().unwrap();
            cache.put(text.to_string(), response.embedding.clone());
        }
        
        Ok(response)
    }

    async fn embed_batch(&self, texts: &[String]) -> EmbeddingResult<Vec<EmbeddingResponse>> {
        let mut results = Vec::with_capacity(texts.len());
        let mut uncached_texts = Vec::new();
        let mut uncached_indices = Vec::new();
        
        // Check which texts are cached
        {
            let mut cache = self.cache.lock().unwrap();
            for (i, text) in texts.iter().enumerate() {
                if let Some(embedding) = cache.get(text.as_str()) {
                    // Cache hit
                    results.push(Some(EmbeddingResponse {
                        embedding: embedding.clone(),
                        tokens: None,
                        model: self.inner.model_name().to_string(),
                    }));
                } else {
                    // Cache miss
                    results.push(None);
                    uncached_texts.push(text.clone());
                    uncached_indices.push(i);
                }
            }
        }
        
        // If all cached, return immediately
        if uncached_texts.is_empty() {
            return Ok(results.into_iter().map(|r| r.unwrap()).collect());
        }
        
        // Fetch uncached embeddings
        let uncached_responses = self.inner.embed_batch(&uncached_texts).await?;
        
        // Store in cache and fill results
        {
            let mut cache = self.cache.lock().unwrap();
            for (idx, response) in uncached_indices.iter().zip(uncached_responses.into_iter()) {
                cache.put(texts[*idx].clone(), response.embedding.clone());
                results[*idx] = Some(response);
            }
        }
        
        Ok(results.into_iter().map(|r| r.unwrap()).collect())
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

// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub size: usize,      // Current number of cached items
    pub capacity: usize,  // Maximum capacity
}

impl CacheStats {
    pub fn hit_rate_estimate(&self) -> f32 {
        if self.capacity == 0 {
            0.0
        } else {
            self.size as f32 / self.capacity as f32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // Mock embedder for testing
    struct MockEmbedder {
        call_count: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl Embedder for MockEmbedder {
        async fn embed(&self, text: &str) -> EmbeddingResult<EmbeddingResponse> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            Ok(EmbeddingResponse {
                embedding: vec![text.len() as f32],  // Simple deterministic embedding
                tokens: Some(1),
                model: "mock".to_string(),
            })
        }

        async fn embed_batch(&self, texts: &[String]) -> EmbeddingResult<Vec<EmbeddingResponse>> {
            let mut responses = Vec::new();
            for text in texts {
                responses.push(self.embed(text).await?);
            }
            Ok(responses)
        }

        fn provider_name(&self) -> &str {
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
    async fn test_cache_hit() {
        let call_count = Arc::new(AtomicUsize::new(0));
        let mock = MockEmbedder { call_count: call_count.clone() };
        let cached = CachedEmbedder::new(mock, 100);

        // First call - should hit API
        cached.embed("hello").await.unwrap();
        assert_eq!(call_count.load(Ordering::SeqCst), 1);

        // Second call - should hit cache
        cached.embed("hello").await.unwrap();
        assert_eq!(call_count.load(Ordering::SeqCst), 1); // Still 1!

        // Different text - should hit API
        cached.embed("world").await.unwrap();
        assert_eq!(call_count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_cache_eviction() {
        let call_count = Arc::new(AtomicUsize::new(0));
        let mock = MockEmbedder { call_count: call_count.clone() };
        let cached = CachedEmbedder::new(mock, 2); // Cache size 2

        // Fill cache
        cached.embed("first").await.unwrap();
        cached.embed("second").await.unwrap();
        assert_eq!(call_count.load(Ordering::SeqCst), 2);

        // Access both - they're already in cache
        cached.embed("first").await.unwrap();
        cached.embed("second").await.unwrap();
        assert_eq!(call_count.load(Ordering::SeqCst), 2); // No new calls

        // Add third - should evict least recently used ("first")
        cached.embed("third").await.unwrap();
        assert_eq!(call_count.load(Ordering::SeqCst), 3);

        // "first" was evicted, "second" and "third" are in cache
        cached.embed("first").await.unwrap(); // Miss - new call
        assert_eq!(call_count.load(Ordering::SeqCst), 4);
        
        cached.embed("second").await.unwrap(); // Miss - was evicted when "first" was added back
        assert_eq!(call_count.load(Ordering::SeqCst), 5);
    }
}
