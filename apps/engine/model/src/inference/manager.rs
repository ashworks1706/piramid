//! The inference domain entry: loads a model from configuration, runs its engine thread, and
//! hands out streamed generations.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::Mutex;
use piramid_core::config::{HardwareConfig, InferenceConfig, SamplingConfig};
use piramid_core::error::InferenceError;
use piramid_core::stats::InferenceMetrics;
use tokio::sync::{mpsc, oneshot};

use crate::fusion::RetrievalHook;
use crate::inference::architecture::{Architecture, ModelSpec};
use crate::inference::batching::request::{FinishReason, GenerationEvent, Usage};
use crate::inference::batching::worker::Command;
use crate::inference::tokenizer::{ChatMessage, ChatTemplate, Tokenizer};

/// A loaded model, its tokenizer and the thread that runs it.
pub struct InferenceManager {
    model_name: String,
    architecture: Architecture,
    device: String,
    tokenizer: Arc<dyn Tokenizer>,
    template: ChatTemplate,
    defaults: SamplingConfig,
    max_sequence_length: usize,
    hook_name: &'static str,
    metrics: Arc<InferenceMetrics>,
    commands: mpsc::UnboundedSender<Command>,
    thread: Mutex<Option<std::thread::JoinHandle<()>>>,
}

impl std::fmt::Debug for InferenceManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InferenceManager")
            .field("model_name", &self.model_name)
            .field("architecture", &self.architecture)
            .field("device", &self.device)
            .field("hook", &self.hook_name)
            .finish_non_exhaustive()
    }
}

/// A generation in flight: its events arrive in order and end with Finished or Failed.
#[derive(Debug)]
pub struct Generation {
    /// Identifier within the engine.
    pub id: u64,
    /// Tokens in the prompt.
    pub prompt_tokens: usize,
    events: mpsc::UnboundedReceiver<GenerationEvent>,
}

/// A generation read to its end.
#[derive(Debug, Clone, PartialEq)]
pub struct Completion {
    /// The generated text.
    pub text: String,
    /// The generated token ids.
    pub tokens: Vec<u32>,
    /// Why it ended.
    pub reason: FinishReason,
    /// Counts and timings.
    pub usage: Usage,
}

impl Generation {
    /// The next event, or None once the stream has ended.
    pub async fn next(&mut self) -> Option<GenerationEvent> {
        self.events.recv().await
    }

    /// Read every event and return the whole completion.
    pub async fn collect(mut self) -> Result<Completion, InferenceError> {
        let mut text = String::new();
        let mut tokens = Vec::new();
        while let Some(event) = self.events.recv().await {
            match event {
                GenerationEvent::Token { token, text: delta } => {
                    tokens.push(token);
                    text.push_str(&delta);
                }
                GenerationEvent::Finished { reason, usage } => {
                    return Ok(Completion {
                        text,
                        tokens,
                        reason,
                        usage,
                    })
                }
                GenerationEvent::Failed(error) => return Err(error),
            }
        }
        Err(InferenceError::Stopped(
            "the engine ended the generation without a result".to_string(),
        ))
    }
}

/// What a caller needs to know about the loaded model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelInfo {
    /// Name of the checkpoint directory.
    pub name: String,
    /// Decoder family.
    pub architecture: &'static str,
    /// Device the model runs on.
    pub device: String,
    /// Longest prompt plus completion.
    pub max_sequence_length: usize,
    /// The retrieval hook the forward pass consults.
    pub hook: &'static str,
}

impl InferenceManager {
    /// Load the model configuration names and start its engine thread.
    ///
    /// Errors when the build has no model runtime, the checkpoint cannot be read, or the device is
    /// unavailable.
    pub fn load(
        config: &InferenceConfig,
        hardware: &HardwareConfig,
        hook: Arc<dyn RetrievalHook>,
    ) -> Result<Self, InferenceError> {
        let dir = checkpoint_dir(config)?;
        let spec = ModelSpec::from_dir(&dir)?;
        if let Some(named) = &config.architecture {
            if Architecture::parse(named)? != spec.architecture {
                return Err(InferenceError::Load(format!(
                    "runtime.inference.architecture is {named} but the checkpoint is {}",
                    spec.architecture.as_str()
                )));
            }
        }
        let template = ChatTemplate::from_dir(&dir)?;
        crate::inference::sampling::validate(&config.sampling)
            .map_err(|e| InferenceError::Load(format!("runtime.inference.sampling: {e}")))?;
        let device = match &config.device {
            Some(device) => device.clone(),
            None if hardware.gpu_enabled() => format!("cuda:{}", hardware.gpu.device_ordinal),
            None => "cpu".to_string(),
        };
        let model_name = dir.file_name().map_or_else(
            || dir.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        );
        let loaded = start(
            config,
            &dir,
            spec,
            &device,
            template.eos_text(),
            hook.clone(),
        )?;
        tracing::info!(
            target: "piramid::inference",
            model = %model_name,
            device = %device,
            kv_blocks = loaded.kv_blocks,
            "model loaded"
        );
        Ok(Self {
            model_name,
            architecture: loaded.architecture,
            device,
            tokenizer: loaded.tokenizer,
            template,
            defaults: config.sampling.clone(),
            max_sequence_length: config.max_sequence_length,
            hook_name: hook.name(),
            metrics: loaded.metrics,
            commands: loaded.commands,
            thread: Mutex::new(Some(loaded.thread)),
        })
    }

    /// What is loaded and where.
    pub fn info(&self) -> ModelInfo {
        ModelInfo {
            name: self.model_name.clone(),
            architecture: self.architecture.as_str(),
            device: self.device.clone(),
            max_sequence_length: self.max_sequence_length,
            hook: self.hook_name,
        }
    }

    /// Sampling settings a request starts from.
    pub fn defaults(&self) -> &SamplingConfig {
        &self.defaults
    }

    /// Render a conversation through the checkpoint's chat template.
    pub fn render_chat(&self, messages: &[ChatMessage]) -> Result<String, InferenceError> {
        self.template.render(messages)
    }

    /// Token ids of prompt text.
    pub fn tokenize(&self, text: &str) -> Result<Vec<u32>, InferenceError> {
        self.tokenizer.encode(text)
    }

    /// Queue a generation. Returns once the engine has admitted or refused it.
    pub async fn generate(
        &self,
        prompt: Vec<u32>,
        sampling: SamplingConfig,
    ) -> Result<Generation, InferenceError> {
        let prompt_tokens = prompt.len();
        let (events, receiver) = mpsc::unbounded_channel();
        let (reply, admitted) = oneshot::channel();
        self.commands
            .send(Command::Submit {
                prompt,
                sampling,
                events,
                reply,
            })
            .map_err(|_| InferenceError::Stopped("the inference engine has stopped".to_string()))?;
        let id = admitted.await.map_err(|_| {
            InferenceError::Stopped("the inference engine has stopped".to_string())
        })??;
        Ok(Generation {
            id,
            prompt_tokens,
            events: receiver,
        })
    }

    /// Counters and gauges of the engine.
    pub fn metrics(&self) -> &InferenceMetrics {
        &self.metrics
    }

    /// Fail every queued and running generation and wait for the engine thread to exit.
    pub fn shutdown(&self) {
        let _ = self.commands.send(Command::Shutdown);
        if let Some(thread) = self.thread.lock().take() {
            if thread.join().is_err() {
                tracing::error!(target: "piramid::inference", "inference engine thread panicked");
            }
        }
    }
}

impl Drop for InferenceManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn checkpoint_dir(config: &InferenceConfig) -> Result<PathBuf, InferenceError> {
    let path = config.model_path.as_deref().ok_or_else(|| {
        InferenceError::Load("runtime.inference.model_path is required".to_string())
    })?;
    let dir = Path::new(path);
    if !dir.is_dir() {
        return Err(InferenceError::Load(format!(
            "runtime.inference.model_path {path} is not a directory"
        )));
    }
    Ok(dir.to_path_buf())
}

struct Started {
    architecture: Architecture,
    tokenizer: Arc<dyn Tokenizer>,
    metrics: Arc<InferenceMetrics>,
    commands: mpsc::UnboundedSender<Command>,
    thread: std::thread::JoinHandle<()>,
    kv_blocks: usize,
}

#[cfg(not(feature = "inference-candle"))]
fn start(
    _config: &InferenceConfig,
    _dir: &Path,
    _spec: ModelSpec,
    _device: &str,
    _eos_text: Option<&str>,
    _hook: Arc<dyn RetrievalHook>,
) -> Result<Started, InferenceError> {
    Err(InferenceError::Unavailable(
        "runtime.inference.enabled needs a build with the inference-candle feature".to_string(),
    ))
}

#[cfg(feature = "inference-candle")]
fn start(
    config: &InferenceConfig,
    dir: &Path,
    spec: ModelSpec,
    device: &str,
    eos_text: Option<&str>,
    hook: Arc<dyn RetrievalHook>,
) -> Result<Started, InferenceError> {
    use std::collections::HashSet;

    use piramid_core::config::{Dtype, Preemption};

    use crate::inference::architecture::{DecoderModel, Precision, StepBatch, StepSequence};
    use crate::inference::backends::candle::runtime::{dtype, DeviceSelection};
    use crate::inference::backends::candle::weights::Weights;
    use crate::inference::backends::candle::{CandleRuntime, QwenModel};
    use crate::inference::backends::tokenizers::JsonTokenizer;
    use crate::inference::batching::scheduler::{Scheduler, SchedulerLimits};
    use crate::inference::batching::worker::{self, WorkerContext};
    use crate::inference::forward::{Driver, SequenceProgress};
    use crate::inference::kv_cache::BlockAllocator;

    if config.kv_cache.preemption == Preemption::Swap {
        return Err(InferenceError::Load(
            "runtime.inference.kv_cache.preemption: swap is not implemented; use recompute"
                .to_string(),
        ));
    }
    let selection = DeviceSelection::parse(device)?;
    let runtime = CandleRuntime::open(selection)?;
    let on_device = runtime.ordinal().is_some();
    let precision = match config.dtype {
        Dtype::Auto if on_device => spec.stored_precision,
        Dtype::Auto | Dtype::Fp32 => Precision::F32,
        Dtype::Fp16 => Precision::F16,
        Dtype::Bf16 => Precision::Bf16,
    };
    let kv_precision = match config.kv_cache.dtype {
        Dtype::Auto => precision,
        Dtype::Fp32 => Precision::F32,
        Dtype::Fp16 => Precision::F16,
        Dtype::Bf16 => Precision::Bf16,
    };

    let tokenizer_path = config
        .tokenizer_path
        .as_deref()
        .map_or_else(|| dir.to_path_buf(), PathBuf::from);
    let tokenizer: Arc<dyn Tokenizer> = Arc::new(JsonTokenizer::load(&tokenizer_path)?);
    let mut eos_token_ids: HashSet<u32> = spec.eos_token_ids.iter().copied().collect();
    if let Some(id) = eos_text.and_then(|text| tokenizer.token_id(text)) {
        eos_token_ids.insert(id);
    }
    let architecture = spec.architecture;

    let weights = Weights::load(dir, runtime.device(), dtype(precision))?;
    let mut model = QwenModel::load(spec, weights, runtime.device(), precision, kv_precision)?;
    runtime.synchronize()?;

    let layout = model.kv_layout();
    let budget = kv_budget(config, runtime.ordinal())?;
    let page_size = config.kv_cache.page_size;
    let blocks = layout.blocks_within(budget, page_size);
    if blocks == 0 {
        return Err(InferenceError::Load(format!(
            "a {budget} byte key/value budget holds no page of {page_size} tokens at {} bytes per token",
            layout.bytes_per_token()
        )));
    }
    model.allocate_cache(blocks * page_size)?;

    let mut driver = Driver::new(model, hook);
    if config.warmup {
        let batch = StepBatch {
            sequences: vec![StepSequence {
                tokens: vec![0],
                start: 0,
                write_slots: vec![0],
                context_slots: vec![0],
                logits: true,
            }],
        };
        let progress = [SequenceProgress {
            tokens: &[0],
            first_step: false,
            finished_chunk: None,
        }];
        driver.step(&batch, &progress)?;
    }

    let batching = &config.batching;
    let limits = SchedulerLimits {
        max_batch_size: batching.max_batch_size,
        max_queue_depth: batching.max_queue_depth,
        max_batched_tokens: batching.max_batched_tokens,
        continuous: batching.continuous,
        chunked_prefill: batching.chunked_prefill,
        prefill_chunk_tokens: batching.prefill_chunk_tokens,
        queue_timeout: batching
            .queue_timeout_ms
            .map(std::time::Duration::from_millis),
        max_sequence_length: config.max_sequence_length,
    };
    let scheduler = Scheduler::new(
        limits,
        BlockAllocator::new(blocks, page_size, config.kv_cache.prefix_sharing),
    );
    let metrics = Arc::new(InferenceMetrics::default());
    let context = WorkerContext {
        tokenizer: tokenizer.clone(),
        eos_token_ids,
        metrics: metrics.clone(),
        chunk_tokens: config.fusion.chunk_tokens,
        max_sequence_length: config.max_sequence_length,
    };
    let (commands, receiver) = mpsc::unbounded_channel();
    let thread = std::thread::Builder::new()
        .name("piramid-inference".to_string())
        .spawn(move || worker::run(driver, scheduler, context, receiver))
        .map_err(|e| InferenceError::Load(format!("inference thread: {e}")))?;
    Ok(Started {
        architecture,
        tokenizer,
        metrics,
        commands,
        thread,
        kv_blocks: blocks,
    })
}

/// Bytes the key/value cache may take: device_fraction of free device memory, capped by
/// max_bytes. On the host, max_bytes is required.
#[cfg(feature = "inference-candle")]
fn kv_budget(config: &InferenceConfig, ordinal: Option<usize>) -> Result<u64, InferenceError> {
    let cache = &config.kv_cache;
    match ordinal {
        #[cfg(feature = "gpu-cuda")]
        Some(ordinal) => {
            let free = piramid_hardware::gpu::Device::open(ordinal)
                .and_then(|device| device.available_memory_bytes())
                .map_err(|e| InferenceError::Load(format!("free device memory: {e}")))?;
            let share = (free as f64 * f64::from(cache.device_fraction)) as u64;
            Ok(cache.max_bytes.map_or(share, |max| max.min(share)))
        }
        #[cfg(not(feature = "gpu-cuda"))]
        Some(ordinal) => Err(InferenceError::Unavailable(format!(
            "cuda:{ordinal} needs a build with the gpu-cuda feature"
        ))),
        None => cache.max_bytes.ok_or_else(|| {
            InferenceError::Load(
                "runtime.inference.kv_cache.max_bytes is required when the model runs on the cpu"
                    .to_string(),
            )
        }),
    }
}
