//! Provider selection: turn an EmbeddingConfig into an Embedder.

use std::num::NonZeroUsize;
use std::str::FromStr;
use std::sync::Arc;

use super::ollama::OllamaEmbedder;
use super::openai::OpenAIEmbedder;
use crate::embeddings::cache::CachedEmbedder;
use crate::embeddings::embedder::{Embedder, EmbeddingResult};
use piramid_core::config::EmbeddingConfig;
use piramid_core::error::embedding::EmbeddingError;

/// Entries kept per embedder.
// The unwrap is evaluated in a const context.
#[allow(clippy::unwrap_used, reason = "const context; checked at compile time")]
const CACHE_CAPACITY: NonZeroUsize = NonZeroUsize::new(10_000).unwrap();

/// Providers this build can construct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingProvider {
    /// Anything speaking the OpenAI embeddings format, including a local server.
    OpenAI,
    /// An Ollama server.
    Ollama,
}

impl FromStr for EmbeddingProvider {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "openai" => Ok(Self::OpenAI),
            "ollama" => Ok(Self::Ollama),
            _ => Err(()),
        }
    }
}

/// Build the embedder named by config, wrapped in the response cache.
pub fn create_embedder(config: &EmbeddingConfig) -> EmbeddingResult<Arc<dyn Embedder>> {
    let provider = config.provider.parse::<EmbeddingProvider>().map_err(|_| {
        EmbeddingError::ConfigError(format!(
            "Unknown provider '{}'. Expected openai or ollama",
            config.provider
        ))
    })?;

    Ok(match provider {
        EmbeddingProvider::OpenAI => Arc::new(CachedEmbedder::new(
            OpenAIEmbedder::new(config)?,
            CACHE_CAPACITY,
        )),
        EmbeddingProvider::Ollama => Arc::new(CachedEmbedder::new(
            OllamaEmbedder::new(config)?,
            CACHE_CAPACITY,
        )),
    })
}
