//! The engine thread: receives requests, plans steps, runs them through the driver, samples and
//! streams tokens, and finishes sequences.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use piramid_core::config::SamplingConfig;
use piramid_core::error::InferenceError;
use piramid_core::stats::{EngineGauges, InferenceMetrics};
use tokio::sync::{mpsc, oneshot};

use crate::inference::architecture::DecoderModel;
use crate::inference::batching::request::{FinishReason, GenerationEvent, Usage};
use crate::inference::batching::scheduler::{Scheduler, Sequence};
use crate::inference::batching::stop::StopMatcher;
use crate::inference::forward::{Driver, SequenceProgress};
use crate::inference::sampling::Sampler;
use crate::inference::tokenizer::{TextStream, Tokenizer};

/// A request for the engine thread.
#[derive(Debug)]
pub enum Command {
    /// Queue a generation; the reply carries its id or why it was refused.
    Submit {
        /// Prompt token ids.
        prompt: Vec<u32>,
        /// Sampling settings, including max_new_tokens and stop strings.
        sampling: SamplingConfig,
        /// Where events go.
        events: mpsc::UnboundedSender<GenerationEvent>,
        /// Admission result.
        reply: oneshot::Sender<Result<u64, InferenceError>>,
    },
    /// Fail every queued and running generation and stop the thread.
    Shutdown,
}

/// Per-sequence state the worker keeps beside the scheduler's.
#[derive(Debug)]
pub struct Caller {
    sampler: Sampler,
    text: TextStream,
    stops: StopMatcher,
    events: mpsc::UnboundedSender<GenerationEvent>,
    first_token: Option<std::time::Duration>,
}

/// What the worker needs besides the driver and scheduler.
pub struct WorkerContext {
    /// Converts sampled tokens to text.
    pub tokenizer: Arc<dyn Tokenizer>,
    /// Token ids that end a generation.
    pub eos_token_ids: HashSet<u32>,
    /// Where counters go.
    pub metrics: Arc<InferenceMetrics>,
    /// Generated tokens per chunk, for the chunk boundary hook point.
    pub chunk_tokens: usize,
    /// Longest prompt plus completion.
    pub max_sequence_length: usize,
}

impl std::fmt::Debug for WorkerContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkerContext")
            .field("eos_token_ids", &self.eos_token_ids)
            .field("chunk_tokens", &self.chunk_tokens)
            .finish_non_exhaustive()
    }
}

/// Run the engine loop until a shutdown command or until every sender is dropped.
pub fn run<M: DecoderModel>(
    mut driver: Driver<M>,
    mut scheduler: Scheduler<Caller>,
    context: WorkerContext,
    mut commands: mpsc::UnboundedReceiver<Command>,
) {
    let mut next_id = 1u64;
    loop {
        let command = if scheduler.is_idle() {
            match commands.blocking_recv() {
                Some(command) => Some(command),
                None => break,
            }
        } else {
            match commands.try_recv() {
                Ok(command) => Some(command),
                Err(mpsc::error::TryRecvError::Empty) => None,
                Err(mpsc::error::TryRecvError::Disconnected) => break,
            }
        };
        if let Some(command) = command {
            match command {
                Command::Submit {
                    prompt,
                    sampling,
                    events,
                    reply,
                } => {
                    let id = next_id;
                    next_id += 1;
                    admit(
                        &mut scheduler,
                        &context,
                        id,
                        prompt,
                        sampling,
                        events,
                        reply,
                    );
                }
                Command::Shutdown => break,
            }
            continue;
        }

        for sequence in scheduler.expire(Instant::now()) {
            fail(
                &context,
                sequence,
                InferenceError::Timeout(
                    "waited longer than queue_timeout_ms for admission".to_string(),
                ),
            );
        }
        cancel_abandoned(&mut scheduler);

        let step = scheduler.plan();
        for _ in &step.preempted {
            context.metrics.record_preemption();
        }
        if step.is_empty() {
            publish_gauges(&scheduler, &context);
            if scheduler.running() == 0 {
                if let Some(sequence) = scheduler.ids().first().and_then(|&id| scheduler.remove(id))
                {
                    fail(
                        &context,
                        sequence,
                        InferenceError::Runtime(
                            "the key/value cache cannot hold this sequence".to_string(),
                        ),
                    );
                }
            }
            continue;
        }

        let progress_tokens: Vec<(Vec<u32>, usize)> = step
            .entries
            .iter()
            .map(|entry| {
                scheduler
                    .running_mut(entry.id)
                    .map(|sequence| (sequence.tokens.clone(), sequence.generated()))
                    .unwrap_or_default()
            })
            .collect();
        let progress: Vec<SequenceProgress<'_>> = step
            .entries
            .iter()
            .zip(&progress_tokens)
            .map(|(entry, (tokens, generated))| SequenceProgress {
                tokens,
                first_step: entry.first_step,
                finished_chunk: (entry.samples
                    && *generated > 0
                    && generated.is_multiple_of(context.chunk_tokens.max(1)))
                .then(|| generated / context.chunk_tokens.max(1) - 1),
            })
            .collect();

        let started = Instant::now();
        let outcome = driver.step(&step.batch, &progress);
        let elapsed = started.elapsed();
        let logits = match outcome {
            Ok(logits) => logits,
            Err(error) => {
                tracing::error!(target: "piramid::inference", %error, "forward step failed");
                for entry in &step.entries {
                    if let Some(sequence) = scheduler.remove(entry.id) {
                        fail(&context, sequence, error.clone());
                    }
                }
                continue;
            }
        };

        let decode_tokens = step
            .entries
            .iter()
            .filter(|e| e.tokens == 1 && !e.first_step)
            .count() as u64;
        let prefill_tokens = step.batch.tokens() as u64 - decode_tokens;
        context.metrics.record_step(
            step.entries.len() as u64,
            prefill_tokens,
            decode_tokens,
            elapsed,
        );

        let mut rows = logits.into_iter();
        for entry in step.entries.iter().filter(|entry| entry.samples) {
            let Some(mut row) = rows.next() else {
                break;
            };
            if let Some(reason) = sample_one(&mut scheduler, &context, entry.id, &mut row) {
                finish(&mut scheduler, &context, entry.id, reason);
            }
        }
        publish_gauges(&scheduler, &context);
    }

    for sequence in scheduler.drain() {
        fail(
            &context,
            sequence,
            InferenceError::Stopped("the inference engine is shutting down".to_string()),
        );
    }
    publish_gauges(&scheduler, &context);
}

fn admit(
    scheduler: &mut Scheduler<Caller>,
    context: &WorkerContext,
    id: u64,
    prompt: Vec<u32>,
    sampling: SamplingConfig,
    events: mpsc::UnboundedSender<GenerationEvent>,
    reply: oneshot::Sender<Result<u64, InferenceError>>,
) {
    let sampler = match Sampler::new(&sampling) {
        Ok(sampler) => sampler,
        Err(error) => {
            let _ = reply.send(Err(error));
            return;
        }
    };
    let prompt_len = prompt.len();
    let caller = Caller {
        sampler,
        text: TextStream::new(),
        stops: StopMatcher::new(&sampling.stop),
        events,
        first_token: None,
    };
    match scheduler.submit(Sequence::new(id, prompt, sampling.max_new_tokens, caller)) {
        Ok(()) => {
            context.metrics.record_admitted(prompt_len as u64);
            let _ = reply.send(Ok(id));
        }
        Err(error) => {
            let _ = reply.send(Err(error));
        }
    }
}

/// Sample the next token of a sequence and stream it. Returns a finish reason when it ends.
fn sample_one(
    scheduler: &mut Scheduler<Caller>,
    context: &WorkerContext,
    id: u64,
    logits: &mut [f32],
) -> Option<FinishReason> {
    let sequence = scheduler.running_mut(id)?;
    let token = match sequence.payload.sampler.sample(logits, &sequence.tokens) {
        Ok(token) => token,
        Err(error) => {
            if let Some(sequence) = scheduler.remove(id) {
                fail(context, sequence, error);
            }
            return None;
        }
    };
    context.metrics.record_generated();
    if sequence.payload.first_token.is_none() {
        let waited = sequence.arrived.elapsed();
        sequence.payload.first_token = Some(waited);
        context.metrics.record_first_token(waited);
        context
            .metrics
            .record_cached_prompt(sequence.cached_prompt_tokens as u64);
    }
    sequence.tokens.push(token);

    if context.eos_token_ids.contains(&token) {
        let text = sequence.payload.stops.flush();
        if !text.is_empty() {
            let _ = sequence
                .payload
                .events
                .send(GenerationEvent::Token { token, text });
        }
        return Some(FinishReason::Stop);
    }
    let delta = match sequence
        .payload
        .text
        .push(context.tokenizer.as_ref(), token)
    {
        Ok(delta) => delta,
        Err(error) => {
            if let Some(sequence) = scheduler.remove(id) {
                fail(context, sequence, error);
            }
            return None;
        }
    };
    let outcome = sequence.payload.stops.push(&delta);
    if outcome.stopped {
        let _ = sequence.payload.events.send(GenerationEvent::Token {
            token,
            text: outcome.text,
        });
        return Some(FinishReason::Stop);
    }
    let at_limit = sequence.generated() >= sequence.max_new_tokens
        || sequence.tokens.len() >= context.max_sequence_length;
    let text = if at_limit {
        outcome.text + &sequence.payload.stops.flush()
    } else {
        outcome.text
    };
    let _ = sequence
        .payload
        .events
        .send(GenerationEvent::Token { token, text });
    at_limit.then_some(FinishReason::Length)
}

fn finish(
    scheduler: &mut Scheduler<Caller>,
    context: &WorkerContext,
    id: u64,
    reason: FinishReason,
) {
    let Some(sequence) = scheduler.remove(id) else {
        return;
    };
    context.metrics.record_finished();
    let usage = usage(&sequence);
    let _ = sequence
        .payload
        .events
        .send(GenerationEvent::Finished { reason, usage });
}

fn fail(context: &WorkerContext, sequence: Sequence<Caller>, error: InferenceError) {
    context.metrics.record_failed();
    let _ = sequence.payload.events.send(GenerationEvent::Failed(error));
}

fn usage(sequence: &Sequence<Caller>) -> Usage {
    Usage {
        prompt_tokens: sequence.prompt_len,
        cached_prompt_tokens: sequence.cached_prompt_tokens,
        completion_tokens: sequence.generated(),
        time_to_first_token: sequence.payload.first_token,
        total_time: sequence.arrived.elapsed(),
    }
}

fn cancel_abandoned(scheduler: &mut Scheduler<Caller>) {
    let abandoned: Vec<u64> = scheduler
        .ids()
        .into_iter()
        .filter(|&id| {
            scheduler
                .running_mut(id)
                .is_some_and(|sequence| sequence.payload.events.is_closed())
        })
        .collect();
    for id in abandoned {
        scheduler.remove(id);
    }
}

fn publish_gauges(scheduler: &Scheduler<Caller>, context: &WorkerContext) {
    let pool = scheduler.pool();
    context.metrics.set_gauges(EngineGauges {
        queue_depth: scheduler.waiting() as u64,
        running: scheduler.running() as u64,
        kv_blocks_total: pool.total_blocks as u64,
        kv_blocks_used: pool.used_blocks as u64,
        kv_blocks_cached: pool.cached_blocks as u64,
        kv_evictions: pool.evictions,
        prefix_hit_tokens: pool.prefix_hit_tokens,
        prefix_lookup_tokens: pool.prefix_lookup_tokens,
    });
}

#[cfg(all(test, feature = "inference-candle"))]
mod tests {
    #![allow(clippy::unwrap_used, reason = "assertions in tests")]

    use std::time::Duration;

    use super::*;
    use crate::fusion::NoopRetrievalHook;
    use crate::inference::architecture::Architecture;
    use crate::inference::backends::candle::qwen::testing::tiny_model;
    use crate::inference::batching::scheduler::SchedulerLimits;
    use crate::inference::kv_cache::BlockAllocator;

    struct Letters;

    impl Tokenizer for Letters {
        fn encode(&self, text: &str) -> Result<Vec<u32>, InferenceError> {
            Ok(text.bytes().map(|b| u32::from(b) % 90).collect())
        }

        fn encode_with_template(&self, text: &str) -> Result<Vec<u32>, InferenceError> {
            self.encode(text)
        }

        fn decode(&self, tokens: &[u32], _skip_special: bool) -> Result<String, InferenceError> {
            Ok(tokens
                .iter()
                .map(|&t| char::from(b'!' + (t % 90) as u8))
                .collect())
        }

        fn token_id(&self, _token: &str) -> Option<u32> {
            None
        }
    }

    fn spawn(
        limits: SchedulerLimits,
        blocks: usize,
        prefix_sharing: bool,
    ) -> (
        mpsc::UnboundedSender<Command>,
        std::thread::JoinHandle<()>,
        Arc<InferenceMetrics>,
    ) {
        let model = tiny_model(Architecture::Qwen3, 11, blocks * 4);
        let driver = Driver::new(model, Arc::new(NoopRetrievalHook));
        let scheduler = Scheduler::new(limits, BlockAllocator::new(blocks, 4, prefix_sharing));
        let metrics = Arc::new(InferenceMetrics::default());
        let context = WorkerContext {
            tokenizer: Arc::new(Letters),
            eos_token_ids: HashSet::new(),
            metrics: metrics.clone(),
            chunk_tokens: 32,
            max_sequence_length: 200,
        };
        let (commands, receiver) = mpsc::unbounded_channel();
        let thread = std::thread::spawn(move || run(driver, scheduler, context, receiver));
        (commands, thread, metrics)
    }

    fn limits() -> SchedulerLimits {
        SchedulerLimits {
            max_batch_size: 8,
            max_queue_depth: 16,
            max_batched_tokens: 64,
            continuous: true,
            chunked_prefill: true,
            prefill_chunk_tokens: 5,
            queue_timeout: Some(Duration::from_secs(30)),
            max_sequence_length: 200,
        }
    }

    fn greedy(max_new_tokens: usize) -> SamplingConfig {
        SamplingConfig {
            max_new_tokens,
            ..SamplingConfig::default()
        }
    }

    async fn generate(
        commands: &mpsc::UnboundedSender<Command>,
        prompt: Vec<u32>,
        sampling: SamplingConfig,
    ) -> Result<(Vec<u32>, FinishReason), InferenceError> {
        let (events, mut receiver) = mpsc::unbounded_channel();
        let (reply, admitted) = oneshot::channel();
        commands
            .send(Command::Submit {
                prompt,
                sampling,
                events,
                reply,
            })
            .unwrap();
        admitted.await.unwrap()?;
        let mut tokens = Vec::new();
        while let Some(event) = receiver.recv().await {
            match event {
                GenerationEvent::Token { token, .. } => tokens.push(token),
                GenerationEvent::Finished { reason, usage } => {
                    assert_eq!(usage.completion_tokens, tokens.len());
                    return Ok((tokens, reason));
                }
                GenerationEvent::Failed(error) => return Err(error),
            }
        }
        Err(InferenceError::Stopped("stream ended".to_string()))
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn concurrent_greedy_generations_match_each_one_alone() {
        let prompts: Vec<Vec<u32>> = vec![(1..12).collect(), vec![40, 41, 42], (20..37).collect()];

        let (commands, thread, _) = spawn(limits(), 64, false);
        let mut alone = Vec::new();
        for prompt in &prompts {
            alone.push(
                generate(&commands, prompt.clone(), greedy(9))
                    .await
                    .unwrap(),
            );
        }
        let together = generate_all(&commands, &prompts, 9).await;
        assert_eq!(together, alone);
        commands.send(Command::Shutdown).unwrap();
        thread.join().unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn preemption_and_prefix_sharing_leave_greedy_output_unchanged() {
        let prompt: Vec<u32> = (5..30).collect();
        let (commands, thread, _) = spawn(limits(), 64, false);
        let reference = generate(&commands, prompt.clone(), greedy(12))
            .await
            .unwrap();
        commands.send(Command::Shutdown).unwrap();
        thread.join().unwrap();

        let (commands, thread, metrics) = spawn(limits(), 12, false);
        let prompts = vec![prompt.clone(), prompt.clone()];
        for result in generate_all(&commands, &prompts, 12).await {
            assert_eq!(result, reference);
        }
        assert!(
            metrics.snapshot().preemptions > 0,
            "{:?}",
            metrics.snapshot()
        );
        commands.send(Command::Shutdown).unwrap();
        thread.join().unwrap();

        let (commands, thread, metrics) = spawn(limits(), 64, true);
        for _ in 0..2 {
            let result = generate(&commands, prompt.clone(), greedy(12))
                .await
                .unwrap();
            assert_eq!(result, reference);
        }
        assert!(
            metrics.snapshot().cached_prompt_tokens > 0,
            "{:?}",
            metrics.snapshot()
        );
        commands.send(Command::Shutdown).unwrap();
        thread.join().unwrap();
    }

    async fn generate_all(
        commands: &mpsc::UnboundedSender<Command>,
        prompts: &[Vec<u32>],
        max_new_tokens: usize,
    ) -> Vec<(Vec<u32>, FinishReason)> {
        let handles: Vec<_> = prompts
            .iter()
            .map(|prompt| {
                let commands = commands.clone();
                let prompt = prompt.clone();
                tokio::spawn(async move {
                    generate(&commands, prompt, greedy(max_new_tokens))
                        .await
                        .unwrap()
                })
            })
            .collect();
        let mut out = Vec::new();
        for handle in handles {
            out.push(handle.await.unwrap());
        }
        out
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn stop_strings_end_a_generation_and_refusals_come_back_at_admission() {
        let (commands, thread, metrics) = spawn(limits(), 64, false);
        let (tokens, reason) = generate(&commands, vec![1, 2, 3], greedy(6)).await.unwrap();
        assert_eq!(reason, FinishReason::Length);
        let text = Letters.decode(&tokens, true).unwrap();
        let stop = text[2..4].to_string();
        let sampling = SamplingConfig {
            stop: vec![stop],
            ..greedy(6)
        };
        let (stopped, reason) = generate(&commands, vec![1, 2, 3], sampling).await.unwrap();
        assert_eq!(reason, FinishReason::Stop);
        assert!(stopped.len() < tokens.len());

        let error = generate(&commands, vec![1; 190], greedy(20))
            .await
            .unwrap_err();
        assert!(
            matches!(error, InferenceError::InvalidRequest(_)),
            "{error}"
        );
        let error = generate(
            &commands,
            vec![1],
            SamplingConfig {
                top_k: Some(0),
                ..greedy(2)
            },
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("top_k"));
        assert_eq!(metrics.snapshot().requests_finished, 2);
        commands.send(Command::Shutdown).unwrap();
        thread.join().unwrap();
    }
}
