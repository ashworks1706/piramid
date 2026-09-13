//! Provider selection: turn an EmbeddingConfig into an Embedder.

use std::num::NonZeroUsize;
use std::sync::Arc;

use super::ollama::OllamaEmbedder;
use super::openai::OpenAIEmbedder;
use crate::embeddings::cache::CachedEmbedder;
use crate::embeddings::embedder::{Embedder, EmbeddingResult};
use piramid_core::config::{EmbeddingConfig, EmbeddingProvider};
use piramid_core::error::embedding::EmbeddingError;

/// Build the embedder named by config, wrapped in the response cache when the config enables it.
pub fn create_embedder(config: &EmbeddingConfig) -> EmbeddingResult<Arc<dyn Embedder>> {
    let capacity = if config.cache.enabled {
        Some(NonZeroUsize::new(config.cache.entries).ok_or_else(|| {
            EmbeddingError::ConfigError(
                "startup.embedding.cache.entries: must be >= 1, or set enabled: false".to_string(),
            )
        })?)
    } else {
        None
    };

    Ok(match (config.provider, capacity) {
        (EmbeddingProvider::OpenAI, Some(capacity)) => {
            Arc::new(CachedEmbedder::new(OpenAIEmbedder::new(config)?, capacity))
        }
        (EmbeddingProvider::OpenAI, None) => Arc::new(OpenAIEmbedder::new(config)?),
        (EmbeddingProvider::Ollama, Some(capacity)) => {
            Arc::new(CachedEmbedder::new(OllamaEmbedder::new(config)?, capacity))
        }
        (EmbeddingProvider::Ollama, None) => Arc::new(OllamaEmbedder::new(config)?),
        (EmbeddingProvider::Piramid, capacity) => piramid_embedder(config, capacity)?,
    })
}

#[cfg(feature = "inference-candle")]
fn piramid_embedder(
    config: &EmbeddingConfig,
    capacity: Option<NonZeroUsize>,
) -> EmbeddingResult<Arc<dyn Embedder>> {
    let embedder = super::piramid::PiramidEmbedder::new(config)?;
    Ok(match capacity {
        Some(capacity) => Arc::new(CachedEmbedder::new(embedder, capacity)),
        None => Arc::new(embedder),
    })
}

#[cfg(not(feature = "inference-candle"))]
fn piramid_embedder(
    _config: &EmbeddingConfig,
    _capacity: Option<NonZeroUsize>,
) -> EmbeddingResult<Arc<dyn Embedder>> {
    Err(EmbeddingError::ConfigError(
        "startup.embedding.provider: piramid needs a build with the inference-candle feature"
            .to_string(),
    ))
}
