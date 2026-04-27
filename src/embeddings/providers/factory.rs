// Provider factory and utilities

use std::sync::Arc;

use crate::embeddings::types::{Embedder, EmbeddingConfig, EmbeddingError, EmbeddingResult};
use super::openai::OpenAIEmbedder;
use super::ollama::OllamaEmbedder;
use super::local::LocalEmbedder;

// Enum of supported embedding providers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingProvider {
    OpenAI,
    Ollama,
    Local,
}

impl EmbeddingProvider {
    // Parse provider from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "openai" => Some(Self::OpenAI),
            "ollama" => Some(Self::Ollama),
            "local" => Some(Self::Local),
            _ => None,
        }
    }

    // Get provider name
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OpenAI => "openai",
            Self::Ollama => "ollama",
            Self::Local => "local",
        }
    }
}

// Create an embedder from configuration
// Determine which embedding provider to use based on the configuration
pub fn create_embedder(config: &EmbeddingConfig) -> EmbeddingResult<Arc<dyn Embedder>> {
    let provider = EmbeddingProvider::from_str(&config.provider).ok_or_else(|| {
        EmbeddingError::ConfigError(format!("Unknown provider: {}", config.provider))
    })?;

    match provider {
        EmbeddingProvider::OpenAI => {
            let embedder = OpenAIEmbedder::new(config)?;
            Ok(Arc::new(embedder))
        }
        EmbeddingProvider::Ollama => {
            let embedder = OllamaEmbedder::new(config)?;
            Ok(Arc::new(embedder))
        }
        EmbeddingProvider::Local => {
            let embedder = LocalEmbedder::new(config)?;
            Ok(Arc::new(embedder))
        }
    }
}
