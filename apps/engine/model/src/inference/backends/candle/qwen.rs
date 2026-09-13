//! The Qwen2 and Qwen3 dense decoders on candle, run one layer at a time with keys and values in a
//! slot-addressed page pool.

use candle_core::{DType, Device, Module, Tensor};
use candle_nn::{Embedding, Linear, RmsNorm};
use piramid_core::error::InferenceError;

use crate::fusion::HiddenState;
use crate::inference::architecture::{
    DecoderModel, HiddenVisitor, ModelSpec, Precision, StepBatch,
};
use crate::inference::backends::candle::runtime::{dtype, runtime};
use crate::inference::backends::candle::weights::Weights;
use crate::inference::kv_cache::KvLayout;

struct Layer {
    q_proj: Linear,
    k_proj: Linear,
    v_proj: Linear,
    o_proj: Linear,
    q_norm: Option<RmsNorm>,
    k_norm: Option<RmsNorm>,
    input_norm: RmsNorm,
    post_attention_norm: RmsNorm,
    gate_proj: Linear,
    up_proj: Linear,
    down_proj: Linear,
}

/// A loaded Qwen2 or Qwen3 model and its key/value page pool.
pub struct QwenModel {
    spec: ModelSpec,
    device: Device,
    dtype: DType,
    kv_dtype: DType,
    kv_precision: Precision,
    embed: Embedding,
    layers: Vec<Layer>,
    norm: RmsNorm,
    lm_head: Linear,
    cos: Tensor,
    sin: Tensor,
    keys: Vec<Tensor>,
    values: Vec<Tensor>,
    #[cfg(feature = "gpu-cuda")]
    gpu: Option<piramid_hardware::gpu::Device>,
}

impl std::fmt::Debug for QwenModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QwenModel")
            .field("architecture", &self.spec.architecture)
            .field("layers", &self.spec.layers)
            .field("dtype", &self.dtype)
            .field("kv_dtype", &self.kv_dtype)
            .finish_non_exhaustive()
    }
}

/// Consecutive tokens of one sequence written to consecutive cache slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WriteRun {
    slot: usize,
    token: usize,
    len: usize,
}

struct SequencePass {
    offset: usize,
    len: usize,
    write_runs: Vec<WriteRun>,
    context: Tensor,
    cos: Tensor,
    sin: Tensor,
    mask: Option<Tensor>,
    logits: bool,
}

/// Hidden states and per-sequence bookkeeping for one forward step.
pub struct QwenPass {
    hidden: Tensor,
    sequences: Vec<SequencePass>,
}

impl std::fmt::Debug for QwenPass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QwenPass")
            .field("sequences", &self.sequences.len())
            .finish_non_exhaustive()
    }
}

impl QwenModel {
    /// Build the model from checkpoint weights. Weights not used by the model are an error.
    pub fn load(
        spec: ModelSpec,
        mut weights: Weights,
        device: &Device,
        precision: Precision,
        kv_precision: Precision,
    ) -> Result<Self, InferenceError> {
        let hidden = spec.hidden_size;
        let head_dim = spec.head_dim;
        let q_width = spec.attention_heads * head_dim;
        let kv_width = spec.kv_heads * head_dim;
        let eps = spec.rms_norm_eps;

        let root = if weights.contains("model.embed_tokens.weight") {
            "model."
        } else {
            ""
        };
        let embed_weight = weights.take(
            &format!("{root}embed_tokens.weight"),
            &[spec.vocab_size, hidden],
        )?;
        let mut layers = Vec::with_capacity(spec.layers);
        for index in 0..spec.layers {
            let prefix = format!("{root}layers.{index}");
            let mut linear = |name: &str, out: usize, inp: usize, bias: bool| {
                let weight = weights.take(&format!("{prefix}.{name}.weight"), &[out, inp])?;
                let bias = if bias {
                    Some(weights.take(&format!("{prefix}.{name}.bias"), &[out])?)
                } else {
                    None
                };
                Ok::<_, InferenceError>(Linear::new(weight, bias))
            };
            let q_proj = linear("self_attn.q_proj", q_width, hidden, spec.qkv_bias)?;
            let k_proj = linear("self_attn.k_proj", kv_width, hidden, spec.qkv_bias)?;
            let v_proj = linear("self_attn.v_proj", kv_width, hidden, spec.qkv_bias)?;
            let o_proj = linear("self_attn.o_proj", hidden, q_width, false)?;
            let gate_proj = linear("mlp.gate_proj", spec.intermediate_size, hidden, false)?;
            let up_proj = linear("mlp.up_proj", spec.intermediate_size, hidden, false)?;
            let down_proj = linear("mlp.down_proj", hidden, spec.intermediate_size, false)?;
            let mut norm = |name: &str, size: usize| {
                Ok::<_, InferenceError>(RmsNorm::new(
                    weights.take(&format!("{prefix}.{name}.weight"), &[size])?,
                    eps,
                ))
            };
            let (q_norm, k_norm) = if spec.qk_norm {
                (
                    Some(norm("self_attn.q_norm", head_dim)?),
                    Some(norm("self_attn.k_norm", head_dim)?),
                )
            } else {
                (None, None)
            };
            layers.push(Layer {
                q_proj,
                k_proj,
                v_proj,
                o_proj,
                q_norm,
                k_norm,
                input_norm: norm("input_layernorm", hidden)?,
                post_attention_norm: norm("post_attention_layernorm", hidden)?,
                gate_proj,
                up_proj,
                down_proj,
            });
        }
        let norm = RmsNorm::new(weights.take(&format!("{root}norm.weight"), &[hidden])?, eps);
        let lm_head = if spec.tie_word_embeddings && !weights.contains("lm_head.weight") {
            Linear::new(embed_weight.clone(), None)
        } else {
            Linear::new(
                weights.take("lm_head.weight", &[spec.vocab_size, hidden])?,
                None,
            )
        };
        let unused = weights.remaining();
        if !unused.is_empty() {
            return Err(InferenceError::Load(format!(
                "checkpoint holds tensors the model does not use: {}",
                unused.join(", ")
            )));
        }

        let model_dtype = dtype(precision);
        let (cos, sin) = rotary_tables(&spec, model_dtype, device)?;
        #[cfg(feature = "gpu-cuda")]
        let gpu = match device.location() {
            candle_core::DeviceLocation::Cuda { gpu_id } => Some(
                piramid_hardware::gpu::Device::open(gpu_id)
                    .map_err(|e| InferenceError::Unavailable(e.to_string()))?,
            ),
            _ => None,
        };
        Ok(Self {
            embed: Embedding::new(embed_weight, hidden),
            spec,
            device: device.clone(),
            dtype: model_dtype,
            kv_dtype: dtype(kv_precision),
            kv_precision,
            layers,
            norm,
            lm_head,
            cos,
            sin,
            keys: Vec::new(),
            values: Vec::new(),
            #[cfg(feature = "gpu-cuda")]
            gpu,
        })
    }

    fn attention(
        &self,
        layer: usize,
        normed: &Tensor,
        pass: &QwenPass,
    ) -> candle_core::Result<Tensor> {
        let weights = &self.layers[layer];
        let (key_store, value_store) = (&self.keys[layer], &self.values[layer]);
        let spec = &self.spec;
        let head_dim = spec.head_dim;
        let groups = spec.attention_heads / spec.kv_heads;
        let q_all = weights.q_proj.forward(normed)?;
        let k_all = weights.k_proj.forward(normed)?;
        let v_all = weights.v_proj.forward(normed)?;
        let scale = 1.0 / (head_dim as f64).sqrt();

        let mut outputs = Vec::with_capacity(pass.sequences.len());
        for sequence in &pass.sequences {
            let len = sequence.len;
            let tokens_first = |tensor: &Tensor, heads: usize| {
                tensor
                    .narrow(1, sequence.offset, len)?
                    .reshape((1, len, heads, head_dim))
            };
            let mut q = tokens_first(&q_all, spec.attention_heads)?
                .transpose(1, 2)?
                .contiguous()?;
            let mut k = tokens_first(&k_all, spec.kv_heads)?;
            if let (Some(q_norm), Some(k_norm)) = (&weights.q_norm, &weights.k_norm) {
                q = q_norm.forward(&q)?;
                k = k_norm.forward(&k)?;
            }
            let q = candle_nn::rotary_emb::rope(&q, &sequence.cos, &sequence.sin)?;
            let k = candle_nn::rotary_emb::rope_thd(&k, &sequence.cos, &sequence.sin)?;

            let k_rows = k.squeeze(0)?.to_dtype(self.kv_dtype)?;
            let v_rows = tokens_first(&v_all, spec.kv_heads)?
                .squeeze(0)?
                .to_dtype(self.kv_dtype)?
                .contiguous()?;
            for run in &sequence.write_runs {
                key_store.slice_set(&k_rows.narrow(0, run.token, run.len)?, 0, run.slot)?;
                value_store.slice_set(&v_rows.narrow(0, run.token, run.len)?, 0, run.slot)?;
            }

            let gather = |store: &Tensor| -> candle_core::Result<Tensor> {
                let rows = store
                    .index_select(&sequence.context, 0)?
                    .to_dtype(self.dtype)?;
                let context = rows.dim(0)?;
                let heads = rows.transpose(0, 1)?.unsqueeze(0)?;
                if groups == 1 {
                    heads.contiguous()
                } else {
                    heads
                        .unsqueeze(2)?
                        .expand((1, spec.kv_heads, groups, context, head_dim))?
                        .reshape((1, spec.attention_heads, context, head_dim))
                }
            };
            let k_context = gather(key_store)?;
            let v_context = gather(value_store)?;

            let mut scores = (q.matmul(&k_context.t()?)? * scale)?;
            if let Some(mask) = &sequence.mask {
                scores = scores.broadcast_add(mask)?;
            }
            let probabilities = candle_nn::ops::softmax_last_dim(&scores)?;
            let mixed = probabilities.matmul(&v_context)?;
            outputs.push(mixed.transpose(1, 2)?.reshape((
                1,
                len,
                spec.attention_heads * head_dim,
            ))?);
        }
        let joined = Tensor::cat(&outputs, 1)?;
        weights.o_proj.forward(&joined)
    }

    fn run_layer(&self, pass: &mut QwenPass, layer: usize) -> candle_core::Result<()> {
        let weights = &self.layers[layer];
        let residual = &pass.hidden;
        let normed = weights.input_norm.forward(residual)?;
        let attended = self.attention(layer, &normed, pass)?;
        let hidden = (residual + attended)?;
        let normed = weights.post_attention_norm.forward(&hidden)?;
        let gate = weights.gate_proj.forward(&normed)?.silu()?;
        let up = weights.up_proj.forward(&normed)?;
        let mlp = weights.down_proj.forward(&(gate * up)?)?;
        pass.hidden = (hidden + mlp)?;
        Ok(())
    }

    /// Hand contiguous f32 rows held on a CUDA device to visit in place, as a borrowed device buffer
    /// covering every element of rows, queued on the per-thread stream candle uses. Returns None on
    /// the CPU, where rows are visited on the host.
    #[cfg(feature = "gpu-cuda")]
    fn device_rows(
        &self,
        rows: &Tensor,
        visit: &mut HiddenVisitor<'_>,
    ) -> Result<Option<()>, InferenceError> {
        use candle_core::cuda_backend::cudarc::driver::DevicePtr;

        let Some(gpu) = &self.gpu else {
            return Ok(None);
        };
        if !rows.is_contiguous() {
            return Err(InferenceError::Runtime(
                "hidden rows handed to a device visitor are not contiguous".to_string(),
            ));
        }
        let (storage, layout) = rows.storage_and_layout();
        let candle_core::Storage::Cuda(cuda) = &*storage else {
            return Err(InferenceError::Runtime(
                "a CUDA model holds hidden states off the device".to_string(),
            ));
        };
        let slice = cuda.as_cuda_slice::<f32>().map_err(runtime)?;
        // The storage guard and the stream record are held until visit returns.
        let (base, _record) = slice.device_ptr(slice.stream());
        let offset = layout
            .start_offset()
            .checked_mul(std::mem::size_of::<f32>())
            .and_then(|bytes| u64::try_from(bytes).ok())
            .ok_or_else(|| {
                InferenceError::Runtime("hidden row offset overflows a device address".to_string())
            })?;
        let mut buffer = piramid_hardware::gpu::DeviceBuffer::<f32>::borrowed(
            gpu,
            base + offset,
            layout.shape().elem_count(),
        )
        .map_err(|e| InferenceError::Runtime(e.to_string()))?;
        let stream = piramid_hardware::gpu::Stream::per_thread(gpu);
        visit(HiddenState::Device(&mut buffer), Some(&stream))?;
        Ok(Some(()))
    }

    #[cfg(not(feature = "gpu-cuda"))]
    fn device_rows(
        &self,
        _rows: &Tensor,
        _visit: &mut HiddenVisitor<'_>,
    ) -> Result<Option<()>, InferenceError> {
        Ok(None)
    }

    /// The normalised last-token hidden state of every sequence that asked for logits, one row per
    /// sequence, or None when none asked.
    fn last_token_states(&self, pass: &QwenPass) -> Result<Option<Tensor>, InferenceError> {
        let last = pass
            .sequences
            .iter()
            .filter(|sequence| sequence.logits)
            .map(|sequence| {
                u32::try_from(sequence.offset + sequence.len - 1).map_err(|_| {
                    InferenceError::Runtime("a step holds more than u32::MAX tokens".to_string())
                })
            })
            .collect::<Result<Vec<u32>, _>>()?;
        if last.is_empty() {
            return Ok(None);
        }
        Tensor::from_slice(&last, last.len(), &self.device)
            .and_then(|index| pass.hidden.squeeze(0)?.index_select(&index, 0))
            .and_then(|rows| self.norm.forward(&rows))
            .map(Some)
            .map_err(runtime)
    }

    fn prepare(&self, batch: &StepBatch) -> Result<QwenPass, InferenceError> {
        if self.keys.is_empty() {
            return Err(InferenceError::Runtime(
                "cache storage is not allocated".to_string(),
            ));
        }
        let slots = self.keys[0].dim(0).map_err(runtime)?;
        let mut tokens = Vec::with_capacity(batch.tokens());
        let mut sequences = Vec::with_capacity(batch.sequences.len());
        for step in &batch.sequences {
            let len = step.tokens.len();
            if len == 0 {
                return Err(InferenceError::Runtime(
                    "a step sequence has no tokens".to_string(),
                ));
            }
            if step.write_slots.len() != len || step.context_slots.len() != step.start + len {
                return Err(InferenceError::Runtime(format!(
                    "a step sequence of {len} tokens at {} has {} write slots and {} context slots",
                    step.start,
                    step.write_slots.len(),
                    step.context_slots.len()
                )));
            }
            if step.start + len > self.spec.max_position_embeddings {
                return Err(InferenceError::InvalidRequest(format!(
                    "position {} exceeds the model limit of {}",
                    step.start + len,
                    self.spec.max_position_embeddings
                )));
            }
            if let Some(&slot) = step
                .context_slots
                .iter()
                .chain(&step.write_slots)
                .find(|&&slot| slot as usize >= slots)
            {
                return Err(InferenceError::Runtime(format!(
                    "slot {slot} is outside the {slots} slot cache"
                )));
            }
            let offset = tokens.len();
            tokens.extend_from_slice(&step.tokens);
            let context =
                Tensor::from_slice(&step.context_slots, step.context_slots.len(), &self.device)
                    .map_err(runtime)?;
            let cos = self.cos.narrow(0, step.start, len).map_err(runtime)?;
            let sin = self.sin.narrow(0, step.start, len).map_err(runtime)?;
            let mask = if len > 1 {
                Some(causal_mask(len, step.start, self.dtype, &self.device).map_err(runtime)?)
            } else {
                None
            };
            sequences.push(SequencePass {
                offset,
                len,
                write_runs: runs(&step.write_slots),
                context,
                cos,
                sin,
                mask,
                logits: step.logits,
            });
        }
        let ids = Tensor::from_slice(&tokens, (1, tokens.len()), &self.device).map_err(runtime)?;
        let hidden = self.embed.forward(&ids).map_err(runtime)?;
        Ok(QwenPass { hidden, sequences })
    }
}

impl DecoderModel for QwenModel {
    type Pass = QwenPass;

    fn spec(&self) -> &ModelSpec {
        &self.spec
    }

    fn kv_layout(&self) -> KvLayout {
        self.spec.kv_layout(self.kv_precision)
    }

    fn allocate_cache(&mut self, slots: usize) -> Result<(), InferenceError> {
        self.keys.clear();
        self.values.clear();
        let shape = (slots, self.spec.kv_heads, self.spec.head_dim);
        let mut keys = Vec::with_capacity(self.spec.layers);
        let mut values = Vec::with_capacity(self.spec.layers);
        for _ in 0..self.spec.layers {
            keys.push(Tensor::zeros(shape, self.kv_dtype, &self.device).map_err(runtime)?);
            values.push(Tensor::zeros(shape, self.kv_dtype, &self.device).map_err(runtime)?);
        }
        self.keys = keys;
        self.values = values;
        Ok(())
    }

    fn begin(&mut self, batch: &StepBatch) -> Result<QwenPass, InferenceError> {
        self.prepare(batch)
    }

    fn layer(&mut self, pass: &mut QwenPass, layer: usize) -> Result<(), InferenceError> {
        if layer >= self.layers.len() {
            return Err(InferenceError::Runtime(format!(
                "layer {layer} of a {} layer model",
                self.layers.len()
            )));
        }
        if layer >= self.keys.len() || layer >= self.values.len() {
            return Err(InferenceError::Runtime(
                "cache storage is not allocated".to_string(),
            ));
        }
        self.run_layer(pass, layer).map_err(runtime)
    }

    fn with_hidden(
        &mut self,
        pass: &mut QwenPass,
        sequence: usize,
        visit: &mut HiddenVisitor<'_>,
    ) -> Result<(), InferenceError> {
        let step = pass.sequences.get(sequence).ok_or_else(|| {
            InferenceError::Runtime(format!("the pass has no sequence {sequence}"))
        })?;
        let (offset, len) = (step.offset, step.len);
        let hidden_size = self.spec.hidden_size;
        let rows = pass
            .hidden
            .narrow(1, offset, len)
            .and_then(|rows| rows.to_dtype(DType::F32))
            .and_then(|rows| rows.contiguous())
            .map_err(runtime)?;
        let replacement = match self.device_rows(&rows, visit)? {
            Some(()) => rows.to_dtype(self.dtype).map_err(runtime)?,
            None => {
                let mut host = rows
                    .flatten_all()
                    .and_then(|rows| rows.to_vec1::<f32>())
                    .map_err(runtime)?;
                visit(HiddenState::Host(&mut host), None)?;
                Tensor::from_vec(host, (1, len, hidden_size), &self.device)
                    .and_then(|rows| rows.to_dtype(self.dtype))
                    .map_err(runtime)?
            }
        };
        let total = pass.hidden.dim(1).map_err(runtime)?;
        if offset == 0 && len == total {
            pass.hidden = replacement;
            return Ok(());
        }
        let before = pass.hidden.narrow(1, 0, offset).map_err(runtime)?;
        let after = pass
            .hidden
            .narrow(1, offset + len, total - offset - len)
            .map_err(runtime)?;
        pass.hidden = Tensor::cat(&[&before, &replacement, &after], 1).map_err(runtime)?;
        Ok(())
    }

    fn finish(&mut self, pass: QwenPass) -> Result<Vec<Vec<f32>>, InferenceError> {
        let Some(normed) = self.last_token_states(&pass)? else {
            return Ok(Vec::new());
        };
        self.lm_head
            .forward(&normed)
            .and_then(|logits| logits.to_dtype(DType::F32))
            .and_then(|logits| logits.to_vec2::<f32>())
            .map_err(runtime)
    }

    fn pool(&mut self, pass: QwenPass) -> Result<Vec<Vec<f32>>, InferenceError> {
        let Some(normed) = self.last_token_states(&pass)? else {
            return Ok(Vec::new());
        };
        normed
            .to_dtype(DType::F32)
            .and_then(|states| states.to_vec2::<f32>())
            .map_err(runtime)
    }
}

fn rotary_tables(
    spec: &ModelSpec,
    dtype: DType,
    device: &Device,
) -> Result<(Tensor, Tensor), InferenceError> {
    let half = spec.head_dim / 2;
    let positions = spec.max_position_embeddings;
    let inverse: Vec<f32> = (0..half)
        .map(|i| 1.0 / spec.rope_theta.powf((2 * i) as f64 / spec.head_dim as f64) as f32)
        .collect();
    let end = u32::try_from(positions).map_err(|_| {
        InferenceError::Load(format!(
            "max_position_embeddings {positions} does not fit a u32 position"
        ))
    })?;
    let build = || -> candle_core::Result<(Tensor, Tensor)> {
        let inverse = Tensor::from_vec(inverse, (1, half), device)?;
        let steps = Tensor::arange(0u32, end, device)?
            .to_dtype(DType::F32)?
            .reshape((positions, 1))?;
        let angles = steps.matmul(&inverse)?;
        Ok((
            angles.cos()?.to_dtype(dtype)?,
            angles.sin()?.to_dtype(dtype)?,
        ))
    };
    build().map_err(runtime)
}

fn causal_mask(
    len: usize,
    start: usize,
    dtype: DType,
    device: &Device,
) -> candle_core::Result<Tensor> {
    let context = start + len;
    let values: Vec<f32> = (0..len)
        .flat_map(|row| {
            (0..context).map(move |column| {
                if column <= start + row {
                    0.0
                } else {
                    f32::NEG_INFINITY
                }
            })
        })
        .collect();
    Tensor::from_vec(values, (len, context), device)?.to_dtype(dtype)
}

/// Contiguous runs of slots, each starting at a slot and a token index.
fn runs(slots: &[u32]) -> Vec<WriteRun> {
    let mut runs: Vec<WriteRun> = Vec::new();
    for (token, &slot) in slots.iter().enumerate() {
        let slot = slot as usize;
        match runs.last_mut() {
            Some(run) if run.slot + run.len == slot && run.token + run.len == token => {
                run.len += 1;
            }
            _ => runs.push(WriteRun {
                slot,
                token,
                len: 1,
            }),
        }
    }
    runs
}

/// Tiny randomly initialised models for driver and scheduler tests.
#[cfg(test)]
pub(crate) mod testing {
    #![allow(clippy::unwrap_used, reason = "test fixtures")]

    use std::collections::HashMap;

    use candle_core::{DType, Device, Tensor};

    use super::QwenModel;
    use crate::inference::architecture::{Architecture, ModelSpec, Precision};
    use crate::inference::backends::candle::weights::Weights;

    /// The spec of a two layer model with a 64 wide residual stream.
    pub(crate) fn tiny_spec(architecture: Architecture) -> ModelSpec {
        ModelSpec {
            architecture,
            vocab_size: 97,
            hidden_size: 64,
            intermediate_size: 128,
            layers: 2,
            attention_heads: 4,
            kv_heads: 2,
            head_dim: 16,
            qkv_bias: architecture == Architecture::Qwen2,
            qk_norm: architecture == Architecture::Qwen3,
            rope_theta: 10_000.0,
            rms_norm_eps: 1e-6,
            max_position_embeddings: 256,
            tie_word_embeddings: true,
            stored_precision: Precision::F32,
            eos_token_ids: vec![96],
        }
    }

    /// Deterministic weights for a spec.
    pub(crate) fn tiny_weights(spec: &ModelSpec, seed: u64) -> HashMap<String, Tensor> {
        let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
        let mut tensor = |shape: &[usize], scale: f32| {
            let count: usize = shape.iter().product();
            let values: Vec<f32> = (0..count)
                .map(|_| {
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    ((state >> 40) as f32 / (1u64 << 24) as f32 - 0.5) * scale
                })
                .collect();
            Tensor::from_vec(values, shape, &Device::Cpu).unwrap()
        };
        let mut map = HashMap::new();
        let h = spec.hidden_size;
        let q = spec.attention_heads * spec.head_dim;
        let kv = spec.kv_heads * spec.head_dim;
        map.insert(
            "model.embed_tokens.weight".to_string(),
            tensor(&[spec.vocab_size, h], 1.0),
        );
        for layer in 0..spec.layers {
            let p = format!("model.layers.{layer}");
            for (name, shape) in [
                ("self_attn.q_proj.weight", vec![q, h]),
                ("self_attn.k_proj.weight", vec![kv, h]),
                ("self_attn.v_proj.weight", vec![kv, h]),
                ("self_attn.o_proj.weight", vec![h, q]),
                ("mlp.gate_proj.weight", vec![spec.intermediate_size, h]),
                ("mlp.up_proj.weight", vec![spec.intermediate_size, h]),
                ("mlp.down_proj.weight", vec![h, spec.intermediate_size]),
            ] {
                map.insert(format!("{p}.{name}"), tensor(&shape, 0.4));
            }
            for (name, size) in [
                ("input_layernorm.weight", h),
                ("post_attention_layernorm.weight", h),
            ] {
                map.insert(format!("{p}.{name}"), (tensor(&[size], 0.2) + 1.0).unwrap());
            }
            if spec.qkv_bias {
                for (name, size) in [("q_proj", q), ("k_proj", kv), ("v_proj", kv)] {
                    map.insert(format!("{p}.self_attn.{name}.bias"), tensor(&[size], 0.1));
                }
            }
            if spec.qk_norm {
                for name in ["q_norm", "k_norm"] {
                    map.insert(
                        format!("{p}.self_attn.{name}.weight"),
                        (tensor(&[spec.head_dim], 0.2) + 1.0).unwrap(),
                    );
                }
            }
        }
        map.insert(
            "model.norm.weight".to_string(),
            (tensor(&[h], 0.2) + 1.0).unwrap(),
        );
        map
    }

    /// A loaded tiny model on the CPU with a cache of slots tokens.
    pub(crate) fn tiny_model(architecture: Architecture, seed: u64, slots: usize) -> QwenModel {
        tiny_model_on(architecture, seed, slots, &Device::Cpu)
    }

    /// A loaded tiny model on a device with a cache of slots tokens.
    pub(crate) fn tiny_model_on(
        architecture: Architecture,
        seed: u64,
        slots: usize,
        device: &Device,
    ) -> QwenModel {
        use crate::inference::architecture::DecoderModel;
        let spec = tiny_spec(architecture);
        let tensors = tiny_weights(&spec, seed)
            .into_iter()
            .map(|(name, tensor)| (name, tensor.to_device(device).unwrap()))
            .collect();
        let weights = Weights::from_tensors(tensors, DType::F32);
        let mut model =
            QwenModel::load(spec, weights, device, Precision::F32, Precision::F32).unwrap();
        model.allocate_cache(slots).unwrap();
        model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::unwrap_used, reason = "assertions in tests")]
    fn rotating_keys_tokens_first_matches_rotating_them_heads_first() {
        let (heads, tokens, dim) = (3, 5, 8);
        let values: Vec<f32> = (0..heads * tokens * dim)
            .map(|i| (i as f32 * 0.37).sin())
            .collect();
        let heads_first = Tensor::from_vec(values, (1, heads, tokens, dim), &Device::Cpu).unwrap();
        let angles: Vec<f32> = (0..tokens * dim / 2).map(|i| i as f32 * 0.11).collect();
        let angles = Tensor::from_vec(angles, (tokens, dim / 2), &Device::Cpu).unwrap();
        let (cos, sin) = (angles.cos().unwrap(), angles.sin().unwrap());

        let expected = candle_nn::rotary_emb::rope(&heads_first, &cos, &sin).unwrap();
        let tokens_first = heads_first.transpose(1, 2).unwrap().contiguous().unwrap();
        let got = candle_nn::rotary_emb::rope_thd(&tokens_first, &cos, &sin)
            .unwrap()
            .transpose(1, 2)
            .unwrap();
        let difference = (expected - got)
            .unwrap()
            .abs()
            .unwrap()
            .max_all()
            .unwrap()
            .to_scalar::<f32>()
            .unwrap();
        assert!(difference < 1e-6, "{difference}");
    }

    #[test]
    fn slots_group_into_contiguous_runs() {
        let run = |slot, token, len| WriteRun { slot, token, len };
        assert_eq!(
            runs(&[4, 5, 6, 12, 13, 0]),
            vec![run(4, 0, 3), run(12, 3, 2), run(0, 5, 1)]
        );
        assert!(runs(&[]).is_empty());
    }

    #[test]
    #[allow(clippy::unwrap_used, reason = "assertions in tests")]
    fn a_layer_without_cache_storage_is_an_error() {
        use crate::inference::architecture::{Architecture, StepSequence};
        let mut model = testing::tiny_model(Architecture::Qwen2, 3, 8);
        let batch = StepBatch {
            sequences: vec![StepSequence {
                tokens: vec![1, 2],
                start: 0,
                write_slots: vec![0, 1],
                context_slots: vec![0, 1],
                logits: true,
            }],
        };
        let mut pass = model.begin(&batch).unwrap();
        model.keys.clear();
        model.values.clear();
        assert!(model.layer(&mut pass, 0).is_err());
    }

    #[test]
    #[allow(clippy::unwrap_used, reason = "assertions in tests")]
    fn a_write_slot_outside_the_cache_is_refused() {
        use crate::inference::architecture::{Architecture, StepSequence};
        let mut model = testing::tiny_model(Architecture::Qwen2, 3, 8);
        let batch = StepBatch {
            sequences: vec![StepSequence {
                tokens: vec![1],
                start: 1,
                write_slots: vec![8],
                context_slots: vec![0, 1],
                logits: true,
            }],
        };
        assert!(model.begin(&batch).is_err());
    }

    #[test]
    #[ignore = "needs PIRAMID_TEST_MODEL pointing at Qwen2.5-0.5B-Instruct"]
    #[allow(
        clippy::unwrap_used,
        clippy::expect_used,
        reason = "assertions in tests"
    )]
    fn the_checkpoint_matches_the_transformers_reference() {
        use crate::inference::architecture::{DecoderModel, ModelSpec, StepBatch, StepSequence};
        use std::path::PathBuf;
        let dir = PathBuf::from(std::env::var("PIRAMID_TEST_MODEL").expect("PIRAMID_TEST_MODEL"));
        let fixture: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/qwen2.5-0.5b-instruct.json"
            ))
            .unwrap(),
        )
        .unwrap();
        let ids = |key: &str| -> Vec<u32> {
            fixture[key]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as u32)
                .collect()
        };
        let prompt = ids("prompt_ids");
        let spec = ModelSpec::from_dir(&dir).unwrap();
        let device = Device::Cpu;
        let weights = Weights::load(&dir, &device, DType::F32).unwrap();
        let mut model =
            QwenModel::load(spec, weights, &device, Precision::F32, Precision::F32).unwrap();
        model.allocate_cache(256).unwrap();
        let mut tokens = prompt.clone();
        let mut generated = Vec::new();
        let mut start = 0;
        for step in 0..8 {
            let end = tokens.len();
            let batch = StepBatch {
                sequences: vec![StepSequence {
                    tokens: tokens[start..end].to_vec(),
                    start,
                    write_slots: (start..end).map(|p| p as u32).collect(),
                    context_slots: (0..end).map(|p| p as u32).collect(),
                    logits: true,
                }],
            };
            let mut pass = model.begin(&batch).unwrap();
            for layer in 0..model.spec().layers {
                model.layer(&mut pass, layer).unwrap();
            }
            let logits = model.finish(pass).unwrap().remove(0);
            if step == 0 {
                let expected = fixture["top5_logits"].as_array().unwrap();
                for (id, value) in ids("top5_ids").iter().zip(expected) {
                    let got = logits[*id as usize];
                    assert!(
                        (got - value.as_f64().unwrap() as f32).abs() < 2e-3,
                        "token {id}: {got}"
                    );
                }
            }
            let next = logits
                .iter()
                .enumerate()
                .fold(0, |best, (i, &v)| if v > logits[best] { i } else { best })
                as u32;
            generated.push(next);
            start = end;
            tokens.push(next);
        }
        assert_eq!(generated, ids("greedy"));
    }

    #[cfg(feature = "gpu-cuda")]
    #[test]
    #[ignore = "needs a CUDA device"]
    #[allow(clippy::unwrap_used, reason = "assertions in tests")]
    fn a_device_kernel_changes_hidden_state_in_place_on_the_model_stream() {
        use crate::fusion::HiddenState;
        use crate::inference::architecture::{Architecture, DecoderModel, StepBatch, StepSequence};
        use piramid_hardware::gpu::{KernelArg, KernelModule, LaunchConfig};

        const SOURCE: &str = r#"
extern "C" __global__ void add_constant(float* rows, unsigned int n, float value) {
    unsigned int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) {
        rows[i] += value;
    }
}
"#;
        let tokens = [3u32, 1, 4, 1, 5];
        let batch = StepBatch {
            sequences: vec![StepSequence {
                tokens: tokens.to_vec(),
                start: 0,
                write_slots: (0..5).collect(),
                context_slots: (0..5).collect(),
                logits: true,
            }],
        };
        let run = |device: &Device| -> (Vec<f32>, &'static str) {
            let mut model = testing::tiny_model_on(Architecture::Qwen3, 5, 16, device);
            let mut pass = model.begin(&batch).unwrap();
            let mut path = "none";
            model
                .with_hidden(&mut pass, 0, &mut |hidden, stream| {
                    match hidden {
                        HiddenState::Host(rows) => {
                            path = "host";
                            for value in rows.iter_mut() {
                                *value += 0.5;
                            }
                        }
                        HiddenState::Device(buffer) => {
                            path = "device";
                            let stream = stream.unwrap();
                            let module = KernelModule::compile(
                                buffer.device(),
                                "add_constant",
                                SOURCE,
                                &["add_constant"],
                            )
                            .unwrap();
                            let n = buffer.len();
                            module
                                .launch(
                                    "add_constant",
                                    LaunchConfig::for_elements(n, 256),
                                    stream,
                                    &[
                                        KernelArg::buffer(buffer),
                                        KernelArg::U32(n as u32),
                                        KernelArg::F32(0.5),
                                    ],
                                )
                                .unwrap();
                        }
                    }
                    Ok(())
                })
                .unwrap();
            for layer in 0..model.spec().layers {
                model.layer(&mut pass, layer).unwrap();
            }
            (model.finish(pass).unwrap().remove(0), path)
        };
        let (host, host_path) = run(&Device::Cpu);
        let (device, device_path) = run(&Device::new_cuda(0).unwrap());
        assert_eq!((host_path, device_path), ("host", "device"));
        for (a, b) in host.iter().zip(&device) {
            assert!((a - b).abs() < 1e-3, "{a} {b}");
        }
    }
}
