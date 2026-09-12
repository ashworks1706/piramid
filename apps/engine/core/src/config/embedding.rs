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

    /// Provider-specific options passed through verbatim.
    #[serde(default)]
    pub options: serde_json::Value,

    /// Request timeout in seconds.
    #[serde(default)]
    pub timeout: Option<u64>,
}

impl EmbeddingConfig {
    /// Reject a provider this build cannot construct.
    pub fn validate(&self) -> Result<(), String> {
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
