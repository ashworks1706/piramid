//! The in-process provider: a Qwen3 embedding checkpoint run by this process's model runtime, on
//! the same device as generation. Last-token pooling, L2-normalised.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::Mutex;
use piramid_core::config::{Dtype, EmbeddingConfig};
use piramid_core::error::embedding::EmbeddingError;
use piramid_core::error::InferenceError;
use serde::Deserialize;

use crate::embeddings::embedder::{Embedder, EmbeddingResponse, EmbeddingResult};
use crate::inference::architecture::{DecoderModel, ModelSpec, StepBatch, StepSequence};
use crate::inference::backends::candle::loader::load_decoder;
use crate::inference::backends::candle::QwenModel;
use crate::inference::backends::tokenizers::JsonTokenizer;
use crate::inference::tokenizer::Tokenizer;

/// Options of the piramid provider, from startup.embedding.options.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PiramidOptions {
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

impl PiramidOptions {
    /// Parse the options object of an embedding configuration.
    pub fn from_config(config: &EmbeddingConfig) -> EmbeddingResult<Self> {
        let parsed = if config.options.is_null() {
            Self::deserialize(&serde_json::json!({}))
        } else {
            Self::deserialize(&config.options)
        }
        .map_err(|e| EmbeddingError::ConfigError(format!("startup.embedding.options: {e}")))?;
        if parsed.max_tokens == 0 {
            return Err(EmbeddingError::ConfigError(
                "startup.embedding.options.max_tokens: must be >= 1".to_string(),
            ));
        }
        Ok(parsed)
    }
}

/// Embeds text with a checkpoint loaded into this process.
pub struct PiramidEmbedder {
    model: Arc<Mutex<QwenModel>>,
    tokenizer: Arc<dyn Tokenizer>,
    name: String,
    dimensions: usize,
    max_tokens: usize,
}

impl std::fmt::Debug for PiramidEmbedder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PiramidEmbedder")
            .field("name", &self.name)
            .field("dimensions", &self.dimensions)
            .finish_non_exhaustive()
    }
}

impl PiramidEmbedder {
    /// Load the checkpoint directory named by config.model.
    pub fn new(config: &EmbeddingConfig) -> EmbeddingResult<Self> {
        if config.base_url.is_some() || config.api_key.is_some() {
            return Err(EmbeddingError::ConfigError(
                "startup.embedding: the piramid provider takes no base_url or api_key".to_string(),
            ));
        }
        let options = PiramidOptions::from_config(config)?;
        let dir = PathBuf::from(&config.model);
        let load = |dir: &Path| -> Result<Self, InferenceError> {
            let spec = ModelSpec::from_dir(dir)?;
            let dimensions = spec.hidden_size;
            let loaded = load_decoder(dir, spec, &options.device, options.dtype, Dtype::Auto)?;
            let mut model = loaded.model;
            model.allocate_cache(options.max_tokens)?;
            Ok(Self {
                model: Arc::new(Mutex::new(model)),
                tokenizer: Arc::new(JsonTokenizer::load(dir)?),
                name: config.model.clone(),
                dimensions,
                max_tokens: options.max_tokens,
            })
        };
        load(&dir).map_err(|e| EmbeddingError::ConfigError(format!("startup.embedding.model: {e}")))
    }
}

/// Run one text through the model and return its pooled, normalised embedding.
fn embed_tokens(model: &mut QwenModel, tokens: Vec<u32>) -> Result<Vec<f32>, InferenceError> {
    let count = u32::try_from(tokens.len()).map_err(|_| {
        InferenceError::InvalidRequest(format!("{} tokens exceed u32::MAX", tokens.len()))
    })?;
    let batch = StepBatch {
        sequences: vec![StepSequence {
            tokens,
            start: 0,
            write_slots: (0..count).collect(),
            context_slots: (0..count).collect(),
            logits: true,
        }],
    };
    let mut pass = model.begin(&batch)?;
    for layer in 0..model.spec().layers {
        model.layer(&mut pass, layer)?;
    }
    let mut vector = model
        .pool(pass)?
        .pop()
        .ok_or_else(|| InferenceError::Runtime("the pass pooled no sequence".to_string()))?;
    normalize(&mut vector);
    Ok(vector)
}

/// Scale a vector to unit length; a zero vector is left as it is.
fn normalize(vector: &mut [f32]) {
    let norm = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in vector {
            *value /= norm;
        }
    }
}

#[async_trait]
impl Embedder for PiramidEmbedder {
    async fn embed(&self, text: &str) -> EmbeddingResult<EmbeddingResponse> {
        let tokens = self
            .tokenizer
            .encode_with_template(text)
            .map_err(|e| EmbeddingError::RequestFailed(e.to_string()))?;
        if tokens.is_empty() {
            return Err(EmbeddingError::RequestFailed(
                "the text has no tokens".to_string(),
            ));
        }
        if tokens.len() > self.max_tokens {
            return Err(EmbeddingError::RequestFailed(format!(
                "the text has {} tokens, more than max_tokens {}",
                tokens.len(),
                self.max_tokens
            )));
        }
        let count = u32::try_from(tokens.len()).map_err(|_| {
            EmbeddingError::RequestFailed(format!("{} tokens exceed u32::MAX", tokens.len()))
        })?;
        let model = Arc::clone(&self.model);
        let embedding =
            tokio::task::spawn_blocking(move || embed_tokens(&mut model.lock(), tokens))
                .await
                .map_err(|e| EmbeddingError::RequestFailed(e.to_string()))?
                .map_err(|e| EmbeddingError::RequestFailed(e.to_string()))?;
        Ok(EmbeddingResponse {
            embedding,
            tokens: Some(count),
            model: self.name.clone(),
        })
    }

    fn provider_name(&self) -> &'static str {
        "piramid"
    }

    fn model_name(&self) -> &str {
        &self.name
    }

    fn dimensions(&self) -> Option<usize> {
        Some(self.dimensions)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "assertions in tests")]

    use super::*;
    use crate::inference::architecture::Architecture;
    use crate::inference::backends::candle::qwen::testing::tiny_model;

    fn config(options: serde_json::Value) -> EmbeddingConfig {
        EmbeddingConfig {
            provider: "piramid".to_string(),
            model: "/models/embed".to_string(),
            api_key: None,
            base_url: None,
            options,
            cache: Default::default(),
            timeout: None,
        }
    }

    #[test]
    fn options_default_and_unknown_keys_are_refused() {
        let parsed = PiramidOptions::from_config(&config(serde_json::Value::Null)).unwrap();
        assert_eq!(parsed.device, "cpu");
        assert_eq!(parsed.max_tokens, 512);
        assert!(
            PiramidOptions::from_config(&config(serde_json::json!({"pooling": "mean"}))).is_err()
        );
        assert!(
            PiramidOptions::from_config(&config(serde_json::json!({"max_tokens": 0}))).is_err()
        );
    }

    #[test]
    fn an_embedding_is_unit_length_and_depends_on_the_text() {
        let mut model = tiny_model(Architecture::Qwen3, 9, 32);
        let a = embed_tokens(&mut model, vec![1, 2, 3]).unwrap();
        let b = embed_tokens(&mut model, vec![1, 2, 4]).unwrap();
        let again = embed_tokens(&mut model, vec![1, 2, 3]).unwrap();
        let norm: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-4);
        assert_eq!(a.len(), 64);
        assert_ne!(a, b);
        for (x, y) in a.iter().zip(&again) {
            assert!((x - y).abs() < 1e-5);
        }
    }
}
