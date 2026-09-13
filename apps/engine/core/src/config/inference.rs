//! Model execution: the forward pass, its memory, and how retrieval enters it.
//!
//! Fusion and document key/value reuse are not implemented; [InferenceConfig::validate] refuses
//! turning them on.

use serde::{Deserialize, Serialize};

/// Model execution and everything under it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct InferenceConfig {
    /// Load a model at startup and serve the generation endpoints.
    pub enabled: bool,

    /// Directory or file holding the weights.
    pub model_path: Option<String>,

    /// Tokenizer location, when it does not sit beside the weights.
    pub tokenizer_path: Option<String>,

    /// Which forked model file drives the pass. None reads it from the checkpoint.
    pub architecture: Option<String>,

    /// Device to load onto: cpu or cuda:N. None is cuda at startup.hardware.gpu.device_ordinal
    /// under the gpu profile and cpu otherwise.
    pub device: Option<String>,

    /// Precision weights are held at. Auto is the checkpoint precision on a GPU and fp32 on the
    /// cpu.
    pub dtype: Dtype,

    /// Longest prompt plus completion, in tokens.
    pub max_sequence_length: usize,

    /// Run a throwaway pass at boot, allocating before the first real request.
    pub warmup: bool,

    /// How requests become forward passes.
    pub batching: BatchingConfig,
    /// Paged key/value cache for attention.
    pub kv_cache: KvCacheConfig,
    /// How the next token is drawn from the logits.
    pub sampling: SamplingConfig,
    /// Retrieval fused into the forward pass.
    pub fusion: FusionConfig,
    /// Precomputed key/value states for retrieved documents.
    pub document_kv: DocumentKvConfig,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        InferenceConfig {
            enabled: false,
            model_path: None,
            tokenizer_path: None,
            architecture: None,
            device: None,
            dtype: Dtype::Auto,
            max_sequence_length: 4096,
            warmup: true,
            batching: BatchingConfig::default(),
            kv_cache: KvCacheConfig::default(),
            sampling: SamplingConfig::default(),
            fusion: FusionConfig::default(),
            document_kv: DocumentKvConfig::default(),
        }
    }
}

/// Numeric precision of weights or cached keys and values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Dtype {
    /// Chosen by the setting that holds it.
    #[default]
    Auto,
    /// 32-bit float.
    Fp32,
    /// 16-bit float.
    Fp16,
    /// 16-bit brain float.
    Bf16,
}

/// How requests become forward passes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct BatchingConfig {
    /// Sequences in one forward pass.
    pub max_batch_size: usize,

    /// Sequences admitted and waiting, across all passes. max_batch_size bounds one step.
    pub max_queue_depth: usize,

    /// Token ceiling per step.
    pub max_batched_tokens: usize,

    /// Keep admitting into a running batch instead of draining it first.
    pub continuous: bool,

    /// Split a long prefill across steps.
    pub chunked_prefill: bool,

    /// Tokens per prefill chunk when chunked_prefill is on.
    pub prefill_chunk_tokens: usize,

    /// How long a request may wait for a slot before it is refused. None waits forever.
    pub queue_timeout_ms: Option<u64>,
}

impl Default for BatchingConfig {
    fn default() -> Self {
        BatchingConfig {
            max_batch_size: 8,
            max_queue_depth: 256,
            max_batched_tokens: 8192,
            continuous: false,
            chunked_prefill: false,
            prefill_chunk_tokens: 2048,
            queue_timeout_ms: Some(30_000),
        }
    }
}

/// Paged key/value cache for attention.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct KvCacheConfig {
    /// Total budget across every live sequence. Required on the cpu; on a GPU it caps the
    /// device_fraction share.
    pub max_bytes: Option<u64>,

    /// Tokens per page. Pages are the unit of allocation and eviction.
    pub page_size: usize,

    /// Precision the cache is held at. Auto is the precision of the weights.
    pub dtype: Dtype,

    /// Fraction of device memory left after weights that the cache may claim.
    pub device_fraction: f32,

    /// Share pages between sequences with an identical prefix, such as a common system prompt.
    pub prefix_sharing: bool,

    /// What happens to a running sequence that loses its pages.
    pub preemption: Preemption,
}

impl Default for KvCacheConfig {
    fn default() -> Self {
        KvCacheConfig {
            max_bytes: None,
            page_size: 16,
            dtype: Dtype::Auto,
            device_fraction: 0.9,
            prefix_sharing: true,
            preemption: Preemption::Recompute,
        }
    }
}

/// What a preempted sequence does with its pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Preemption {
    /// Drop them and recompute from the prompt when the sequence resumes.
    #[default]
    Recompute,
    /// Copy them to host memory and back. Not implemented.
    Swap,
}

/// How the next token is drawn from the logits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct SamplingConfig {
    /// Draw temperature. 0.0 is greedy.
    pub temperature: f32,

    /// Nucleus cutoff. None disables it.
    pub top_p: Option<f32>,

    /// Keep only this many highest-probability tokens. None disables it.
    pub top_k: Option<usize>,

    /// Penalty on tokens already produced. 1.0 is no penalty.
    pub repetition_penalty: f32,

    /// How far back the repetition penalty looks.
    pub repetition_window: usize,

    /// Default completion length when a request does not say.
    pub max_new_tokens: usize,

    /// Strings that end a generation.
    pub stop: Vec<String>,

    /// Fixed seed for the sampler. None is nondeterministic.
    pub seed: Option<u64>,
}

impl Default for SamplingConfig {
    fn default() -> Self {
        SamplingConfig {
            temperature: 0.0,
            top_p: None,
            top_k: None,
            repetition_penalty: 1.0,
            repetition_window: 64,
            max_new_tokens: 512,
            stop: Vec::new(),
            seed: None,
        }
    }
}

/// Retrieval fused into the forward pass, through piramid_model::fusion::RetrievalHook.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct FusionConfig {
    /// Call the retrieval hook during generation.
    pub enabled: bool,

    /// Which hook implementation to install, by registered name.
    pub hook: Option<String>,

    /// Which collection the hook queries. None uses the one named by the request.
    pub collection: Option<String>,

    /// Where in the pass retrieval may fire.
    pub point: RetrievalPointKind,

    /// Neighbours fetched per hook call.
    pub top_k: usize,

    /// Candidates fetched before filtering, as a multiple of top_k.
    pub overfetch: usize,

    /// Drop neighbours scoring below this. None keeps whatever the index returns.
    pub score_threshold: Option<f32>,

    /// Tokens between calls when point is chunk-boundary.
    pub chunk_tokens: usize,

    /// Fire every Nth decoder layer when point is layer-entry.
    pub layer_stride: usize,

    /// Hard ceiling on hook calls per generated sequence. None is unlimited.
    pub max_calls_per_sequence: Option<usize>,

    /// Abandon a retrieval that has not returned within this budget. None always waits.
    pub deadline_us: Option<u64>,

    /// What a missed deadline does.
    pub on_deadline_miss: DeadlineMiss,

    /// Run retrieval on its own device stream.
    pub own_stream: bool,

    /// Suppress a document already fused earlier in the same sequence.
    pub dedupe_across_calls: bool,

    /// Tokens the prompt-stuffed control arm may spend.
    pub token_budget: Option<usize>,

    /// Store token ids for retrieved documents.
    pub pretokenize_documents: bool,

    /// Trained adapter weights, for architectures that have not been forked.
    pub adapter_path: Option<String>,
}

impl Default for FusionConfig {
    fn default() -> Self {
        FusionConfig {
            enabled: false,
            hook: None,
            collection: None,
            point: RetrievalPointKind::SequenceStart,
            top_k: 8,
            overfetch: 1,
            score_threshold: None,
            chunk_tokens: 32,
            layer_stride: 1,
            max_calls_per_sequence: None,
            deadline_us: Some(100),
            on_deadline_miss: DeadlineMiss::Skip,
            own_stream: true,
            dedupe_across_calls: true,
            token_budget: None,
            pretokenize_documents: true,
            adapter_path: None,
        }
    }
}

/// Where in the forward pass a hook may run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum RetrievalPointKind {
    /// Once, before the first token.
    #[default]
    SequenceStart,
    /// Every chunk_tokens generated tokens.
    ChunkBoundary,
    /// Between decoder layers, every layer_stride of them.
    LayerEntry,
}

/// What happens when retrieval misses its deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum DeadlineMiss {
    /// Continue the pass without the neighbours.
    #[default]
    Skip,
    /// Block until retrieval returns.
    Stall,
    /// Fail the request.
    Error,
}

/// Precomputed key/value states for retrieved documents, reused at prefill.
///
/// Position and preceding context change these states, so they are not concatenated as they
/// stand. recompute_ratio is the fraction repaired on reuse.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct DocumentKvConfig {
    /// Store and reuse document states at prefill.
    pub enabled: bool,

    /// Budget for the store, drawn from the same device memory as the live KV cache.
    pub max_bytes: Option<u64>,

    /// Where the states are held.
    pub storage: DocumentKvStorage,

    /// Fraction of a reused document's tokens recomputed to repair context dependence.
    pub recompute_ratio: f32,
}

impl Default for DocumentKvConfig {
    fn default() -> Self {
        DocumentKvConfig {
            enabled: false,
            max_bytes: None,
            storage: DocumentKvStorage::Host,
            recompute_ratio: 0.15,
        }
    }
}

/// Where precomputed document states live.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum DocumentKvStorage {
    /// Device memory.
    Device,
    /// Host memory.
    #[default]
    Host,
    /// A sidecar beside the collection.
    Disk,
}

impl InferenceConfig {
    /// Reject anything the build cannot honour.
    ///
    /// Turning on fusion or document key/value reuse is an error naming the roadmap version that
    /// will implement it.
    pub fn validate(&self) -> Result<(), String> {
        if self.enabled && self.model_path.is_none() {
            return Err("runtime.inference.model_path: required when inference is enabled".into());
        }
        if let Some(device) = &self.device {
            let cuda = device
                .strip_prefix("cuda:")
                .is_some_and(|ordinal| ordinal.parse::<usize>().is_ok());
            if device != "cpu" && !cuda {
                return Err(format!(
                    "runtime.inference.device: {device} is not cpu or cuda:N"
                ));
            }
        }
        if let Some(name) = &self.architecture {
            if name != "qwen2" && name != "qwen3" {
                return Err(format!(
                    "runtime.inference.architecture: {name} is not qwen2 or qwen3"
                ));
            }
        }
        if self.max_sequence_length == 0 {
            return Err("runtime.inference.max_sequence_length: must be >= 1".into());
        }
        if self.kv_cache.preemption == Preemption::Swap {
            return Err(
                "runtime.inference.kv_cache.preemption: swap is not implemented yet; use recompute"
                    .into(),
            );
        }
        if self.fusion.enabled {
            return Err(
                "runtime.inference.fusion.enabled: not implemented yet (roadmap v0.6.0)".into(),
            );
        }
        if self.document_kv.enabled {
            return Err(
                "runtime.inference.document_kv.enabled: not implemented yet (roadmap v0.6.0)"
                    .into(),
            );
        }

        if self.batching.max_batch_size == 0 {
            return Err("runtime.inference.batching.max_batch_size: must be >= 1".into());
        }
        if self.batching.max_queue_depth < self.batching.max_batch_size {
            return Err(
                "runtime.inference.batching.max_queue_depth: must be >= max_batch_size".into(),
            );
        }
        if self.kv_cache.page_size == 0 {
            return Err("runtime.inference.kv_cache.page_size: must be >= 1".into());
        }
        if !(0.0..=1.0).contains(&self.kv_cache.device_fraction) {
            return Err(
                "runtime.inference.kv_cache.device_fraction: must be within 0.0..=1.0".into(),
            );
        }
        if self.fusion.top_k == 0 {
            return Err("runtime.inference.fusion.top_k: must be >= 1".into());
        }
        if self.fusion.overfetch == 0 {
            return Err("runtime.inference.fusion.overfetch: must be >= 1".into());
        }
        if self.fusion.chunk_tokens == 0 {
            return Err("runtime.inference.fusion.chunk_tokens: must be >= 1".into());
        }
        if self.fusion.layer_stride == 0 {
            return Err("runtime.inference.fusion.layer_stride: must be >= 1".into());
        }
        if !(0.0..=1.0).contains(&self.document_kv.recompute_ratio) {
            return Err(
                "runtime.inference.document_kv.recompute_ratio: must be within 0.0..=1.0".into(),
            );
        }
        if self.batching.max_batched_tokens == 0 {
            return Err("runtime.inference.batching.max_batched_tokens: must be >= 1".into());
        }
        if self.batching.prefill_chunk_tokens == 0 {
            return Err("runtime.inference.batching.prefill_chunk_tokens: must be >= 1".into());
        }
        if self.sampling.repetition_penalty <= 0.0 {
            return Err("runtime.inference.sampling.repetition_penalty: must be > 0".into());
        }
        if self.sampling.temperature < 0.0 {
            return Err("runtime.inference.sampling.temperature: must be >= 0".into());
        }
        if self.sampling.top_p.is_some_and(|p| !(p > 0.0 && p <= 1.0)) {
            return Err("runtime.inference.sampling.top_p: must be within (0.0, 1.0]".into());
        }
        if self.sampling.top_k == Some(0) {
            return Err("runtime.inference.sampling.top_k: must be >= 1".into());
        }
        if self.sampling.max_new_tokens == 0 {
            return Err("runtime.inference.sampling.max_new_tokens: must be >= 1".into());
        }
        Ok(())
    }
}
