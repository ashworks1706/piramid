//! Model architectures and the contract a backend implements to run one: [ModelSpec] is what a
//! checkpoint declares, [DecoderModel] is one loaded model driven layer by layer.

use std::path::Path;

use piramid_core::error::InferenceError;
use serde::Deserialize;

use crate::fusion::HiddenState;
use crate::inference::kv_cache::KvLayout;
use piramid_hardware::gpu::Stream;

/// The decoder families this build can run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Architecture {
    /// Qwen2 and Qwen2.5: biased query, key and value projections.
    Qwen2,
    /// Qwen3 dense: unbiased projections and per-head query and key normalisation.
    Qwen3,
}

impl Architecture {
    /// The model_type string a checkpoint names this architecture by.
    pub fn as_str(&self) -> &'static str {
        match self {
            Architecture::Qwen2 => "qwen2",
            Architecture::Qwen3 => "qwen3",
        }
    }

    /// Parse a model_type string.
    pub fn parse(name: &str) -> Result<Self, InferenceError> {
        match name {
            "qwen2" => Ok(Architecture::Qwen2),
            "qwen3" => Ok(Architecture::Qwen3),
            other => Err(InferenceError::Load(format!(
                "architecture {other} is not supported; expected qwen2 or qwen3"
            ))),
        }
    }
}

/// Precision a checkpoint or cache holds its floats at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Precision {
    /// 32-bit float.
    F32,
    /// 16-bit float.
    F16,
    /// 16-bit brain float.
    Bf16,
}

impl Precision {
    /// Bytes per element.
    pub fn bytes(&self) -> usize {
        match self {
            Precision::F32 => 4,
            Precision::F16 | Precision::Bf16 => 2,
        }
    }

    fn from_torch_dtype(name: &str) -> Result<Self, InferenceError> {
        match name {
            "float32" => Ok(Precision::F32),
            "float16" => Ok(Precision::F16),
            "bfloat16" => Ok(Precision::Bf16),
            other => Err(InferenceError::Load(format!(
                "checkpoint torch_dtype {other} is not supported"
            ))),
        }
    }
}

/// What a checkpoint declares about its shape, read from config.json.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelSpec {
    /// Decoder family.
    pub architecture: Architecture,
    /// Vocabulary entries in the embedding and output projection.
    pub vocab_size: usize,
    /// Width of the residual stream.
    pub hidden_size: usize,
    /// Width of the feed-forward inner layer.
    pub intermediate_size: usize,
    /// Decoder layers.
    pub layers: usize,
    /// Query heads.
    pub attention_heads: usize,
    /// Key and value heads.
    pub kv_heads: usize,
    /// Width of one attention head.
    pub head_dim: usize,
    /// Whether the query, key and value projections carry a bias.
    pub qkv_bias: bool,
    /// Whether queries and keys are RMS-normalised per head before rotation.
    pub qk_norm: bool,
    /// Rotary embedding base.
    pub rope_theta: f64,
    /// RMS normalisation epsilon.
    pub rms_norm_eps: f64,
    /// Longest position the checkpoint was trained for.
    pub max_position_embeddings: usize,
    /// Whether the output projection reuses the embedding matrix.
    pub tie_word_embeddings: bool,
    /// Precision the weights are stored at.
    pub stored_precision: Precision,
    /// Token ids that end a generation, from config.json and generation_config.json.
    pub eos_token_ids: Vec<u32>,
}

#[derive(Debug, Deserialize)]
struct RawConfig {
    model_type: String,
    vocab_size: usize,
    hidden_size: usize,
    intermediate_size: usize,
    num_hidden_layers: usize,
    num_attention_heads: usize,
    num_key_value_heads: Option<usize>,
    head_dim: Option<usize>,
    attention_bias: Option<bool>,
    rope_theta: Option<f64>,
    rms_norm_eps: f64,
    max_position_embeddings: usize,
    #[serde(default)]
    tie_word_embeddings: bool,
    hidden_act: Option<String>,
    #[serde(default)]
    use_sliding_window: bool,
    torch_dtype: Option<String>,
    eos_token_id: Option<TokenIds>,
    rope_scaling: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct RawGenerationConfig {
    eos_token_id: Option<TokenIds>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum TokenIds {
    One(u32),
    Many(Vec<u32>),
}

impl TokenIds {
    fn into_vec(self) -> Vec<u32> {
        match self {
            TokenIds::One(id) => vec![id],
            TokenIds::Many(ids) => ids,
        }
    }
}

impl ModelSpec {
    /// Read config.json, and generation_config.json when present, from a checkpoint directory.
    pub fn from_dir(dir: &Path) -> Result<Self, InferenceError> {
        let config = read(&dir.join("config.json"))?;
        let generation = match std::fs::read_to_string(dir.join("generation_config.json")) {
            Ok(text) => Some(text),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => {
                return Err(InferenceError::Load(format!(
                    "generation_config.json: {error}"
                )))
            }
        };
        Self::from_json(&config, generation.as_deref())
    }

    /// Parse the text of config.json and, optionally, generation_config.json.
    pub fn from_json(config: &str, generation: Option<&str>) -> Result<Self, InferenceError> {
        let raw: RawConfig = serde_json::from_str(config)
            .map_err(|e| InferenceError::Load(format!("config.json: {e}")))?;
        let architecture = Architecture::parse(&raw.model_type)?;
        if raw.use_sliding_window {
            return Err(InferenceError::Load(
                "config.json: use_sliding_window is not supported".to_string(),
            ));
        }
        if raw
            .rope_scaling
            .as_ref()
            .is_some_and(|value| !value.is_null())
        {
            return Err(InferenceError::Load(
                "config.json: rope_scaling is not supported".to_string(),
            ));
        }
        if let Some(act) = raw.hidden_act.as_deref() {
            if act != "silu" {
                return Err(InferenceError::Load(format!(
                    "config.json: hidden_act {act} is not supported; expected silu"
                )));
            }
        }
        if raw.num_attention_heads == 0 {
            return Err(InferenceError::Load(
                "config.json: num_attention_heads is zero".to_string(),
            ));
        }
        let kv_heads = raw.num_key_value_heads.unwrap_or(raw.num_attention_heads);
        if kv_heads == 0 || !raw.num_attention_heads.is_multiple_of(kv_heads) {
            return Err(InferenceError::Load(format!(
                "config.json: {} attention heads are not a multiple of {kv_heads} key/value heads",
                raw.num_attention_heads
            )));
        }
        let head_dim = raw
            .head_dim
            .unwrap_or(raw.hidden_size / raw.num_attention_heads);
        let (qkv_bias, qk_norm) = match architecture {
            Architecture::Qwen2 => (true, false),
            Architecture::Qwen3 => (raw.attention_bias.unwrap_or(false), true),
        };
        let mut eos_token_ids = raw.eos_token_id.map(TokenIds::into_vec).unwrap_or_default();
        if let Some(text) = generation {
            let generation: RawGenerationConfig = serde_json::from_str(text)
                .map_err(|e| InferenceError::Load(format!("generation_config.json: {e}")))?;
            for id in generation
                .eos_token_id
                .map(TokenIds::into_vec)
                .unwrap_or_default()
            {
                if !eos_token_ids.contains(&id) {
                    eos_token_ids.push(id);
                }
            }
        }
        Ok(Self {
            architecture,
            vocab_size: raw.vocab_size,
            hidden_size: raw.hidden_size,
            intermediate_size: raw.intermediate_size,
            layers: raw.num_hidden_layers,
            attention_heads: raw.num_attention_heads,
            kv_heads,
            head_dim,
            qkv_bias,
            qk_norm,
            rope_theta: raw.rope_theta.unwrap_or(10_000.0),
            rms_norm_eps: raw.rms_norm_eps,
            max_position_embeddings: raw.max_position_embeddings,
            tie_word_embeddings: raw.tie_word_embeddings,
            stored_precision: match raw.torch_dtype.as_deref() {
                Some(name) => Precision::from_torch_dtype(name)?,
                None => Precision::F32,
            },
            eos_token_ids,
        })
    }

    /// Weight elements the checkpoint holds for this architecture, counting a tied output
    /// projection once.
    pub fn parameter_count(&self) -> u64 {
        let h = self.hidden_size as u64;
        let head_dim = self.head_dim as u64;
        let q = self.attention_heads as u64 * head_dim;
        let kv = self.kv_heads as u64 * head_dim;
        let inner = self.intermediate_size as u64;
        let vocab = self.vocab_size as u64;
        let bias = if self.qkv_bias { q + 2 * kv } else { 0 };
        let qk_norm = if self.qk_norm { 2 * head_dim } else { 0 };
        let layer = h * q + 2 * h * kv + q * h + bias + 3 * h * inner + 2 * h + qk_norm;
        let head = if self.tie_word_embeddings {
            0
        } else {
            vocab * h
        };
        vocab * h + self.layers as u64 * layer + h + head
    }

    /// Cache shape for this model at a given element precision.
    pub fn kv_layout(&self, precision: Precision) -> KvLayout {
        KvLayout {
            layers: self.layers,
            kv_heads: self.kv_heads,
            head_dim: self.head_dim,
            bytes_per_element: precision.bytes(),
        }
    }
}

fn read(path: &Path) -> Result<String, InferenceError> {
    std::fs::read_to_string(path)
        .map_err(|e| InferenceError::Load(format!("{}: {e}", path.display())))
}

/// One sequence's part of a forward step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepSequence {
    /// Tokens computed in this step.
    pub tokens: Vec<u32>,
    /// Position of the first token.
    pub start: usize,
    /// Cache slot each token's key and value are written to, one per token.
    pub write_slots: Vec<u32>,
    /// Cache slots of every position from zero through the last token, in order.
    pub context_slots: Vec<u32>,
    /// Whether the step returns logits for the last token.
    pub logits: bool,
}

/// Every sequence computed in one forward step.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StepBatch {
    /// Sequences in the order their logits are returned.
    pub sequences: Vec<StepSequence>,
}

impl StepBatch {
    /// Tokens computed across the batch.
    pub fn tokens(&self) -> usize {
        self.sequences.iter().map(|s| s.tokens.len()).sum()
    }
}

/// Receives the hidden states of one sequence, and the device stream when they live on a device.
pub type HiddenVisitor<'v> =
    dyn FnMut(HiddenState<'_>, Option<&Stream>) -> Result<(), InferenceError> + 'v;

/// A loaded model that a driver runs one decoder layer at a time.
pub trait DecoderModel: Send {
    /// State carried through one forward step: hidden states and per-step bookkeeping.
    type Pass;

    /// What the checkpoint declares.
    fn spec(&self) -> &ModelSpec;

    /// Shape of the cache as this model stores it.
    fn kv_layout(&self) -> KvLayout;

    /// Allocate cache storage for slots tokens, replacing any held before.
    fn allocate_cache(&mut self, slots: usize) -> Result<(), InferenceError>;

    /// Embed the batch's tokens into hidden states.
    fn begin(&mut self, batch: &StepBatch) -> Result<Self::Pass, InferenceError>;

    /// Run one decoder layer over the pass, writing its keys and values to the cache.
    fn layer(&mut self, pass: &mut Self::Pass, layer: usize) -> Result<(), InferenceError>;

    /// Hand the hidden states of one sequence of the pass to visit, which may change them. On a
    /// device, visit also receives the stream the model's work is queued on.
    fn with_hidden(
        &mut self,
        pass: &mut Self::Pass,
        sequence: usize,
        visit: &mut HiddenVisitor<'_>,
    ) -> Result<(), InferenceError>;

    /// Normalise and project, returning the last-token logits of every sequence that asked.
    fn finish(&mut self, pass: Self::Pass) -> Result<Vec<Vec<f32>>, InferenceError>;

    /// Normalise, returning the last-token hidden state of every sequence that asked, without the
    /// output projection.
    fn pool(&mut self, pass: Self::Pass) -> Result<Vec<Vec<f32>>, InferenceError>;
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "assertions in tests")]

    use super::*;

    const QWEN25: &str = r#"{
        "architectures": ["Qwen2ForCausalLM"], "model_type": "qwen2", "vocab_size": 151936,
        "hidden_size": 896, "intermediate_size": 4864, "num_hidden_layers": 24,
        "num_attention_heads": 14, "num_key_value_heads": 2, "rope_theta": 1000000.0,
        "rms_norm_eps": 1e-06, "max_position_embeddings": 32768, "tie_word_embeddings": true,
        "hidden_act": "silu", "use_sliding_window": false, "torch_dtype": "bfloat16",
        "eos_token_id": 151643, "sliding_window": 32768, "rope_scaling": null
    }"#;

    #[test]
    fn a_qwen25_config_reads_into_a_spec() {
        let spec =
            ModelSpec::from_json(QWEN25, Some(r#"{"eos_token_id": [151645, 151643]}"#)).unwrap();
        assert_eq!(spec.architecture, Architecture::Qwen2);
        assert_eq!(spec.head_dim, 64);
        assert!(spec.qkv_bias && !spec.qk_norm);
        assert_eq!(spec.stored_precision, Precision::Bf16);
        assert_eq!(spec.eos_token_ids, vec![151643, 151645]);
        assert_eq!(spec.kv_layout(Precision::Bf16).bytes_per_token(), 12_288);
        assert_eq!(spec.parameter_count(), 494_032_768);
    }

    #[test]
    fn unsupported_checkpoints_are_refused_by_name() {
        for (field, value) in [
            ("model_type", r#""llama""#),
            ("use_sliding_window", "true"),
            ("hidden_act", r#""gelu""#),
            ("rope_scaling", r#"{"type": "yarn"}"#),
        ] {
            let mut json: serde_json::Value = serde_json::from_str(QWEN25).unwrap();
            json[field] = serde_json::from_str(value).unwrap();
            let error = ModelSpec::from_json(&json.to_string(), None).unwrap_err();
            let needle = if field == "model_type" {
                "llama"
            } else {
                field
            };
            assert!(error.to_string().contains(needle), "{error}");
        }
    }
}
