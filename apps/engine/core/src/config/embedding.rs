//! Embedding provider configuration.

use serde::{Deserialize, Serialize};

/// How to reach an embedding provider.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbeddingConfig {
    /// Provider name: openai, including any server speaking that wire format, or ollama.
    pub provider: String,

    /// Model identifier as the provider understands it.
    pub model: String,

    /// API key. OPENAI_API_KEY sets it from the environment.
    #[serde(default)]
    pub api_key: Option<String>,

    /// Base URL, for self-hosted or proxied endpoints.
    #[serde(default)]
    pub base_url: Option<String>,

    /// Extra request fields. For openai they are merged into the request body; for ollama they
    /// are sent as the options object of the request. Null or an object.
    #[serde(default)]
    pub options: serde_json::Value,

    /// Embeddings kept so identical text is not sent to the provider twice.
    #[serde(default)]
    pub cache: super::EmbeddingCacheConfig,

    /// Request timeout in seconds.
    #[serde(default)]
    pub timeout: Option<u64>,
}

impl EmbeddingConfig {
    pub fn validate(&self) -> Result<(), String> {
        self.cache.validate()?;
        match &self.options {
            serde_json::Value::Null => {}
            serde_json::Value::Object(fields) => {
                if self.provider == "openai" {
                    if let Some(reserved) = ["model", "input", "encoding_format"]
                        .into_iter()
                        .find(|name| fields.contains_key(*name))
                    {
                        return Err(format!(
                            "startup.embedding.options: '{reserved}' is set by the provider and \
                             cannot be overridden"
                        ));
                    }
                }
            }
            _ => return Err("startup.embedding.options: must be null or a mapping".into()),
        }
        match self.provider.as_str() {
            "openai" | "ollama" => Ok(()),
            "piramid" => {
                Err("startup.embedding.provider: 'piramid' is not implemented yet (roadmap v0.4.0)"
                    .into())
            }
            other => Err(format!(
                "startup.embedding.provider: unknown provider '{other}', expected 'openai' or 'ollama'"
            )),
        }
    }
}
