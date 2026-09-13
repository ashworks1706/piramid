//! Embedding provider configuration.

use serde::{Deserialize, Serialize};

/// How to reach an embedding provider.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbeddingConfig {
    /// Provider name: openai, including any server speaking that wire format, ollama, or piramid
    /// for a checkpoint run by this process.
    pub provider: String,

    /// Model identifier as the provider understands it; for piramid, the checkpoint directory.
    pub model: String,

    /// API key. OPENAI_API_KEY sets it from the environment.
    #[serde(default)]
    pub api_key: Option<String>,

    /// Base URL, for self-hosted or proxied endpoints.
    #[serde(default)]
    pub base_url: Option<String>,

    /// Extra request fields. For openai they are merged into the request body; for ollama they
    /// are sent as the options object of the request; for piramid they are device, dtype and
    /// max_tokens. Null or an object.
    #[serde(default)]
    pub options: serde_json::Value,

    /// Cache of embeddings keyed by input text.
    #[serde(default)]
    pub cache: super::EmbeddingCacheConfig,

    /// Request timeout in seconds.
    #[serde(default)]
    pub timeout: Option<u64>,
}

impl EmbeddingConfig {
    /// Reject a provider this build cannot construct.
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
                if self.base_url.is_some() {
                    return Err("startup.embedding.base_url: the piramid provider takes none".into());
                }
                if let serde_json::Value::Object(fields) = &self.options {
                    if let Some(unknown) = fields
                        .keys()
                        .find(|key| !["device", "dtype", "max_tokens"].contains(&key.as_str()))
                    {
                        return Err(format!(
                            "startup.embedding.options: '{unknown}' is not a piramid option; expected device, dtype or max_tokens"
                        ));
                    }
                }
                Ok(())
            }
            other => Err(format!(
                "startup.embedding.provider: unknown provider '{other}', expected 'openai', 'ollama' or 'piramid'"
            )),
        }
    }
}
