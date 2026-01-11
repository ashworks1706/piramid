//! Provider factory and utilities

use std::sync::Arc;

use super::{Embedder, EmbeddingConfig, EmbeddingError, EmbeddingResult};
use super::openai::OpenAIEmbedder;
use super::ollama::OllamaEmbedder;

/// Enum of supported embedding providers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingProvider {
    OpenAI,
    Ollama,
}

impl EmbeddingProvider {
    /// Parse provider from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "openai" => Some(Self::OpenAI),
            "ollama" => Some(Self::Ollama),
            _ => None,
        }
    }

    /// Get provider name
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OpenAI => "openai",
            Self::Ollama => "ollama",
        }
    }
}

/// Create an embedder from configuration
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_from_str() {
        assert_eq!(EmbeddingProvider::from_str("openai"), Some(EmbeddingProvider::OpenAI));
        assert_eq!(EmbeddingProvider::from_str("OpenAI"), Some(EmbeddingProvider::OpenAI));
        assert_eq!(EmbeddingProvider::from_str("ollama"), Some(EmbeddingProvider::Ollama));
        assert_eq!(EmbeddingProvider::from_str("unknown"), None);
    }

    #[test]
    fn test_provider_as_str() {
        assert_eq!(EmbeddingProvider::OpenAI.as_str(), "openai");
        assert_eq!(EmbeddingProvider::Ollama.as_str(), "ollama");
    }
}
