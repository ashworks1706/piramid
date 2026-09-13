//! Generation counters: requests, tokens, time to first token, decode speed, and the state of the
//! scheduler and key/value cache.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Running totals and gauges of the inference engine, shared across threads.
#[derive(Debug, Default)]
pub struct InferenceMetrics {
    requests_admitted: AtomicU64,
    requests_finished: AtomicU64,
    requests_failed: AtomicU64,
    prompt_tokens: AtomicU64,
    cached_prompt_tokens: AtomicU64,
    generated_tokens: AtomicU64,
    first_token_count: AtomicU64,
    first_token_ns: AtomicU64,
    decode_steps: AtomicU64,
    decode_tokens: AtomicU64,
    decode_ns: AtomicU64,
    prefill_tokens: AtomicU64,
    prefill_ns: AtomicU64,
    preemptions: AtomicU64,
    queue_depth: AtomicU64,
    running: AtomicU64,
    last_batch_size: AtomicU64,
    kv_blocks_total: AtomicU64,
    kv_blocks_used: AtomicU64,
    kv_blocks_cached: AtomicU64,
    kv_evictions: AtomicU64,
    prefix_hit_tokens: AtomicU64,
    prefix_lookup_tokens: AtomicU64,
}

/// The values of an [InferenceMetrics] at one moment. Averages are None before their first sample.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InferenceMetricsSnapshot {
    /// Requests accepted into the queue.
    pub requests_admitted: u64,
    /// Requests that ended with a finish reason.
    pub requests_finished: u64,
    /// Requests that ended with an error.
    pub requests_failed: u64,
    /// Prompt tokens across admitted requests.
    pub prompt_tokens: u64,
    /// Prompt tokens served from shared cache pages.
    pub cached_prompt_tokens: u64,
    /// Tokens sampled across all requests.
    pub generated_tokens: u64,
    /// Mean time from admission to the first sampled token, in milliseconds.
    pub avg_time_to_first_token_ms: Option<f32>,
    /// Decode tokens per second across all decode work.
    pub decode_tokens_per_second: Option<f32>,
    /// Mean duration of a decode step, in milliseconds.
    pub avg_decode_step_ms: Option<f32>,
    /// Prefill tokens per second across all prefill work.
    pub prefill_tokens_per_second: Option<f32>,
    /// Sequences whose cache pages were taken back.
    pub preemptions: u64,
    /// Requests waiting for admission.
    pub queue_depth: u64,
    /// Sequences being generated.
    pub running: u64,
    /// Sequences in the most recent forward step.
    pub last_batch_size: u64,
    /// Pages in the key/value pool.
    pub kv_blocks_total: u64,
    /// Pages held by a sequence.
    pub kv_blocks_used: u64,
    /// Free pages that still carry a reusable prefix.
    pub kv_blocks_cached: u64,
    /// Prefix pages evicted to make room.
    pub kv_evictions: u64,
    /// Share of looked-up prompt tokens served from shared pages.
    pub prefix_hit_rate: Option<f32>,
}

/// Pool and scheduler gauges written after every step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EngineGauges {
    /// Requests waiting for admission.
    pub queue_depth: u64,
    /// Sequences being generated.
    pub running: u64,
    /// Pages in the key/value pool.
    pub kv_blocks_total: u64,
    /// Pages held by a sequence.
    pub kv_blocks_used: u64,
    /// Free pages that still carry a reusable prefix.
    pub kv_blocks_cached: u64,
    /// Prefix pages evicted since load.
    pub kv_evictions: u64,
    /// Prompt tokens served from shared pages since load.
    pub prefix_hit_tokens: u64,
    /// Prompt tokens looked up for sharing since load.
    pub prefix_lookup_tokens: u64,
}

impl InferenceMetrics {
    /// Count an admitted request and its prompt tokens.
    pub fn record_admitted(&self, prompt_tokens: u64) {
        self.requests_admitted.fetch_add(1, Ordering::Relaxed);
        self.prompt_tokens
            .fetch_add(prompt_tokens, Ordering::Relaxed);
    }

    /// Count prompt tokens a sequence took from shared pages.
    pub fn record_cached_prompt(&self, tokens: u64) {
        self.cached_prompt_tokens
            .fetch_add(tokens, Ordering::Relaxed);
    }

    /// Count a sampled token.
    pub fn record_generated(&self) {
        self.generated_tokens.fetch_add(1, Ordering::Relaxed);
    }

    /// Record the wait from admission to a request's first token.
    pub fn record_first_token(&self, elapsed: Duration) {
        self.first_token_count.fetch_add(1, Ordering::Relaxed);
        self.first_token_ns
            .fetch_add(saturating_nanos(elapsed), Ordering::Relaxed);
    }

    /// Record one forward step: how many tokens were prefill and decode, and how long it took.
    /// The time is split between the two in proportion to their tokens.
    pub fn record_step(
        &self,
        batch_size: u64,
        prefill_tokens: u64,
        decode_tokens: u64,
        elapsed: Duration,
    ) {
        let total = u128::from(prefill_tokens) + u128::from(decode_tokens);
        if total == 0 {
            return;
        }
        let ns = saturating_nanos(elapsed);
        let prefill_ns =
            u64::try_from(u128::from(ns) * u128::from(prefill_tokens) / total).unwrap_or(ns);
        self.last_batch_size.store(batch_size, Ordering::Relaxed);
        self.prefill_tokens
            .fetch_add(prefill_tokens, Ordering::Relaxed);
        self.prefill_ns.fetch_add(prefill_ns, Ordering::Relaxed);
        if decode_tokens > 0 {
            self.decode_steps.fetch_add(1, Ordering::Relaxed);
            self.decode_tokens
                .fetch_add(decode_tokens, Ordering::Relaxed);
            self.decode_ns.fetch_add(ns - prefill_ns, Ordering::Relaxed);
        }
    }

    /// Count a finished request.
    pub fn record_finished(&self) {
        self.requests_finished.fetch_add(1, Ordering::Relaxed);
    }

    /// Count a failed request.
    pub fn record_failed(&self) {
        self.requests_failed.fetch_add(1, Ordering::Relaxed);
    }

    /// Count a preempted sequence.
    pub fn record_preemption(&self) {
        self.preemptions.fetch_add(1, Ordering::Relaxed);
    }

    /// Overwrite the scheduler and pool gauges.
    pub fn set_gauges(&self, gauges: EngineGauges) {
        self.queue_depth
            .store(gauges.queue_depth, Ordering::Relaxed);
        self.running.store(gauges.running, Ordering::Relaxed);
        self.kv_blocks_total
            .store(gauges.kv_blocks_total, Ordering::Relaxed);
        self.kv_blocks_used
            .store(gauges.kv_blocks_used, Ordering::Relaxed);
        self.kv_blocks_cached
            .store(gauges.kv_blocks_cached, Ordering::Relaxed);
        self.kv_evictions
            .store(gauges.kv_evictions, Ordering::Relaxed);
        self.prefix_hit_tokens
            .store(gauges.prefix_hit_tokens, Ordering::Relaxed);
        self.prefix_lookup_tokens
            .store(gauges.prefix_lookup_tokens, Ordering::Relaxed);
    }

    /// Read the current values.
    pub fn snapshot(&self) -> InferenceMetricsSnapshot {
        let load = |counter: &AtomicU64| counter.load(Ordering::Relaxed);
        let per_second =
            |tokens: u64, ns: u64| (ns > 0).then(|| (tokens as f64 * 1e9 / ns as f64) as f32);
        let mean_ms =
            |ns: u64, count: u64| (count > 0).then(|| (ns as f64 / count as f64 / 1e6) as f32);
        let lookups = load(&self.prefix_lookup_tokens);
        InferenceMetricsSnapshot {
            requests_admitted: load(&self.requests_admitted),
            requests_finished: load(&self.requests_finished),
            requests_failed: load(&self.requests_failed),
            prompt_tokens: load(&self.prompt_tokens),
            cached_prompt_tokens: load(&self.cached_prompt_tokens),
            generated_tokens: load(&self.generated_tokens),
            avg_time_to_first_token_ms: mean_ms(
                load(&self.first_token_ns),
                load(&self.first_token_count),
            ),
            decode_tokens_per_second: per_second(load(&self.decode_tokens), load(&self.decode_ns)),
            avg_decode_step_ms: mean_ms(load(&self.decode_ns), load(&self.decode_steps)),
            prefill_tokens_per_second: per_second(
                load(&self.prefill_tokens),
                load(&self.prefill_ns),
            ),
            preemptions: load(&self.preemptions),
            queue_depth: load(&self.queue_depth),
            running: load(&self.running),
            last_batch_size: load(&self.last_batch_size),
            kv_blocks_total: load(&self.kv_blocks_total),
            kv_blocks_used: load(&self.kv_blocks_used),
            kv_blocks_cached: load(&self.kv_blocks_cached),
            kv_evictions: load(&self.kv_evictions),
            prefix_hit_rate: (lookups > 0)
                .then(|| load(&self.prefix_hit_tokens) as f32 / lookups as f32),
        }
    }
}

/// Nanoseconds in a duration, clamped to u64::MAX.
fn saturating_nanos(elapsed: Duration) -> u64 {
    u64::try_from(elapsed.as_nanos()).unwrap_or(u64::MAX)
}
