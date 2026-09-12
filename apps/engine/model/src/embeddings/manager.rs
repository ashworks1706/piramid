//! The embeddings domain entry.

use std::sync::Arc;

use crate::embeddings::embedder::{Embedder, EmbeddingResult};
use crate::embeddings::providers::create_embedder;
use crate::embeddings::retry::RetryEmbedder;
use piramid_core::config::EmbeddingConfig;
use piramid_core::stats::EmbedMetrics;

/// Owns the embedding stack and its throughput counters.
///
/// One of these lives in AppState, and holds every field the embedding domain owns.
pub struct EmbeddingsManager {
    embedder: Option<Arc<dyn Embedder>>,
    metrics: EmbedMetrics,
}

impl EmbeddingsManager {
    /// A manager with no provider configured; every embed request reports unavailable.
    pub fn disabled() -> Self {
        Self {
            embedder: None,
            metrics: EmbedMetrics::default(),
        }
    }

    /// Wrap an embedder the caller built in the retry layer.
    ///
    /// The seam for a provider this crate cannot construct. The binary builds the embedder and
    /// passes it here.
    pub fn with_embedder(embedder: Arc<dyn Embedder>) -> Self {
        Self {
            embedder: Some(Arc::new(RetryEmbedder::new(embedder))),
            metrics: EmbedMetrics::default(),
        }
    }

    /// Build the full stack config names: provider, response cache, retries.
    pub fn from_config(config: &EmbeddingConfig) -> EmbeddingResult<Self> {
        let embedder = create_embedder(config)?;
        Ok(Self {
            embedder: Some(Arc::new(RetryEmbedder::new(embedder))),
            metrics: EmbedMetrics::default(),
        })
    }

    /// Whether a provider is configured.
    pub fn is_configured(&self) -> bool {
        self.embedder.is_some()
    }

    /// The configured embedder, if any.
    pub fn embedder(&self) -> Option<&Arc<dyn Embedder>> {
        self.embedder.as_ref()
    }

    /// Throughput counters for the embedding path.
    pub fn metrics(&self) -> &EmbedMetrics {
        &self.metrics
    }
}
