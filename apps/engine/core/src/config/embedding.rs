//! Embedding provider configuration.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::{DeviceSelection, Dtype};

/// Endpoint the openai provider posts to when base_url is unset.
pub const DEFAULT_OPENAI_BASE_URL: &str = "https://api.openai.com/v1/embeddings";

/// Server root the ollama provider calls when base_url is unset.
pub const DEFAULT_OLLAMA_BASE_URL: &str = "http://localhost:11434";

/// Request fields the openai provider sets, which options cannot override.
const OPENAI_RESERVED_OPTIONS: [&str; 3] = ["model", "input", "encoding_format"];

/// How to reach an embedding provider.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbeddingConfig {
    /// Which provider embeds text.
    pub provider: EmbeddingProvider,

    /// Model identifier as the provider understands it; for piramid, the checkpoint directory.
    pub model: String,

    /// API key for the openai provider. Set from OPENAI_API_KEY only.
    #[serde(skip)]
    pub api_key: Option<String>,

    /// Base URL, for self-hosted or proxied endpoints.
    #[serde(default)]
    pub base_url: Option<String>,

    /// Extra request fields, merged into the provider's request. Null or an object.
    #[serde(default)]
    pub options: serde_json::Value,

    /// Cache of embeddings keyed by input text.
    #[serde(default)]
    pub cache: EmbeddingCacheConfig,

    /// Request timeout in seconds.
    #[serde(default)]
    pub timeout: Option<u64>,
}

/// The embedding providers a configuration can name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EmbeddingProvider {
    /// Anything speaking the OpenAI embeddings format, including a local server.
    OpenAI,
    /// An Ollama server.
    Ollama,
    /// A checkpoint run by this process; needs the inference-candle feature.
    Piramid,
}

impl EmbeddingProvider {
    /// The name the configuration file uses.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenAI => "openai",
            Self::Ollama => "ollama",
            Self::Piramid => "piramid",
        }
    }
}

impl fmt::Display for EmbeddingProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Options of the piramid provider, from startup.embedding.options.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PiramidEmbeddingOptions {
    /// cpu or cuda:N.
    #[serde(default = "default_device")]
    pub device: String,
    /// auto, fp32, fp16 or bf16.
    #[serde(default)]
    pub dtype: Dtype,
    /// Longest text in tokens; longer texts are refused.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
}

fn default_device() -> String {
    "cpu".to_string()
}

fn default_max_tokens() -> usize {
    512
}

impl EmbeddingConfig {
    /// The options as request fields: the object, or no fields for null.
    pub fn request_options(&self) -> Result<serde_json::Map<String, serde_json::Value>, String> {
        match &self.options {
            serde_json::Value::Null => Ok(serde_json::Map::new()),
            serde_json::Value::Object(fields) => Ok(fields.clone()),
            _ => Err("startup.embedding.options: must be null or a mapping".into()),
        }
    }

    /// The options parsed as those of the piramid provider.
    pub fn piramid_options(&self) -> Result<PiramidEmbeddingOptions, String> {
        let fields = serde_json::Value::Object(self.request_options()?);
        let options = PiramidEmbeddingOptions::deserialize(&fields)
            .map_err(|e| format!("startup.embedding.options: {e}"))?;
        DeviceSelection::parse(&options.device)
            .map_err(|e| format!("startup.embedding.options.device: {e}"))?;
        if options.max_tokens == 0 {
            return Err("startup.embedding.options.max_tokens: must be >= 1".into());
        }
        Ok(options)
    }

    /// Reject a setting the named provider does not take.
    pub fn validate(&self) -> Result<(), String> {
        self.cache.validate()?;
        if self.timeout == Some(0) {
            return Err("startup.embedding.timeout: must be >= 1".into());
        }
        let fields = self.request_options()?;
        match self.provider {
            EmbeddingProvider::OpenAI => {
                if let Some(reserved) = OPENAI_RESERVED_OPTIONS
                    .into_iter()
                    .find(|name| fields.contains_key(*name))
                {
                    return Err(format!(
                        "startup.embedding.options: '{reserved}' is set by the provider and \
                         cannot be overridden"
                    ));
                }
                Ok(())
            }
            EmbeddingProvider::Ollama => {
                if self.api_key.is_some() {
                    return Err("startup.embedding.api_key: the ollama provider takes none".into());
                }
                Ok(())
            }
            EmbeddingProvider::Piramid => {
                if self.base_url.is_some() {
                    return Err(
                        "startup.embedding.base_url: the piramid provider takes none".into(),
                    );
                }
                if self.api_key.is_some() {
                    return Err("startup.embedding.api_key: the piramid provider takes none".into());
                }
                if self.timeout.is_some() {
                    return Err("startup.embedding.timeout: the piramid provider takes none".into());
                }
                self.piramid_options().map(|_| ())
            }
        }
    }
}

/// Cache of embeddings keyed by input text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct EmbeddingCacheConfig {
    /// Whether embeddings are cached at all.
    pub enabled: bool,

    /// Entry ceiling.
    pub entries: usize,
}

impl Default for EmbeddingCacheConfig {
    fn default() -> Self {
        EmbeddingCacheConfig {
            enabled: true,
            entries: 10_000,
        }
    }
}

impl EmbeddingCacheConfig {
    /// Reject a cache that is on with no room.
    pub fn validate(&self) -> Result<(), String> {
        if self.enabled && self.entries == 0 {
            return Err(
                "startup.embedding.cache.entries: must be >= 1, or set enabled: false".into(),
            );
        }
        Ok(())
    }
}
