//! The engine thread: plans steps, runs the driver, samples, streams tokens, finishes sequences.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use piramid_core::config::SamplingConfig;
use piramid_core::error::InferenceError;
use piramid_core::stats::{EngineGauges, InferenceMetrics};
use tokio::sync::{mpsc, oneshot};

use crate::inference::architecture::DecoderModel;
use crate::inference::batching::request::{FinishReason, GenerationEvent, Usage};
use crate::inference::batching::scheduler::{PlannedEntry, Scheduler, Sequence};
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

        let mut step = scheduler.plan();
        for _ in &step.preempted {
            context.metrics.record_preemption();
        }
        let failed = std::mem::take(&mut step.failed);
        let any_failed = !failed.is_empty();
        for (sequence, error) in failed {
            tracing::error!(target: "piramid::inference", %error, id = sequence.id, "planning a sequence failed");
            fail(&context, sequence, error);
        }
        if step.is_empty() && any_failed {
            publish_gauges(&scheduler, &context);
            continue;
        }
        if step.is_empty() {
            if let Some(sequence) = scheduler.ids().first().and_then(|&id| scheduler.remove(id)) {
                fail(
                    &context,
                    sequence,
                    InferenceError::Runtime(
                        "the key/value cache cannot hold this sequence".to_string(),
                    ),
                );
            }
            publish_gauges(&scheduler, &context);
            continue;
        }

        let started = Instant::now();
        let outcome = {
            let chunk = context.chunk_tokens.max(1);
            step_progress(&scheduler, &step.entries, chunk)
                .and_then(|progress| driver.step(&step.batch, &progress))
        };
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
                if let Some(sequence) = scheduler.remove(entry.id) {
                    fail(
                        &context,
                        sequence,
                        InferenceError::Runtime(
                            "the forward step returned no logits for this sequence".to_string(),
                        ),
                    );
                }
                continue;
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

/// Where each planned sequence stands. Errors when a planned sequence is not running.
pub fn step_progress<'s>(
    scheduler: &'s Scheduler<Caller>,
    entries: &[PlannedEntry],
    chunk: usize,
) -> Result<Vec<SequenceProgress<'s>>, InferenceError> {
    entries
        .iter()
        .map(|entry| {
            let sequence = scheduler.running(entry.id).ok_or_else(|| {
                InferenceError::Runtime(format!("planned sequence {} is not running", entry.id))
            })?;
            let generated = sequence.generated();
            Ok(SequenceProgress {
                tokens: sequence.tokens.as_slice(),
                first_step: entry.first_step,
                finished_chunk: (entry.samples && generated > 0 && generated.is_multiple_of(chunk))
                    .then(|| generated / chunk - 1),
            })
        })
        .collect()
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
    scheduler.remove_where(|sequence| sequence.payload.events.is_closed());
}

fn publish_gauges(scheduler: &Scheduler<Caller>, context: &WorkerContext) {
    let pool = scheduler.pool();
    context.metrics.set_gauges(EngineGauges {
        queue_depth: scheduler.waiting() as u64,
        running: scheduler.running_count() as u64,
        kv_blocks_total: pool.total_blocks as u64,
        kv_blocks_used: pool.used_blocks as u64,
        kv_blocks_cached: pool.cached_blocks as u64,
        kv_evictions: pool.evictions,
        prefix_hit_tokens: pool.prefix_hit_tokens,
        prefix_lookup_tokens: pool.prefix_lookup_tokens,
    });
}
