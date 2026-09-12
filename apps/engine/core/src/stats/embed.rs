//! Embedding throughput counters.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Running totals of embedding requests, shared across threads.
#[derive(Default)]
pub struct EmbedMetrics {
    requests: AtomicU64,
    texts: AtomicU64,
    total_tokens: AtomicU64,
    total_latency_ns: AtomicU64,
}

/// The totals of an [EmbedMetrics] at one moment.
#[derive(Debug, Clone, Copy)]
pub struct EmbedMetricsSnapshot {
    /// Embedding requests recorded.
    pub requests: u64,
    /// Texts embedded across all requests.
    pub texts: u64,
    /// Tokens the providers reported across all requests.
    pub total_tokens: u64,
    /// Mean latency per request in milliseconds. None before any request is recorded.
    pub avg_latency_ms: Option<f32>,
}

impl EmbedMetrics {
    /// Add counts and elapsed time to the totals.
    pub fn record(&self, request_count: u64, text_count: u64, token_count: u64, latency: Duration) {
        self.requests.fetch_add(request_count, Ordering::Relaxed);
        self.texts.fetch_add(text_count, Ordering::Relaxed);
        self.total_tokens.fetch_add(token_count, Ordering::Relaxed);
        self.total_latency_ns
            .fetch_add(latency.as_nanos() as u64, Ordering::Relaxed);
    }

    /// Read the current totals.
    pub fn snapshot(&self) -> EmbedMetricsSnapshot {
        let requests = self.requests.load(Ordering::Relaxed);
        let total_latency_ns = self.total_latency_ns.load(Ordering::Relaxed);
        let avg_latency_ms = if requests > 0 {
            Some((total_latency_ns as f64 / requests as f64 / 1_000_000.0) as f32)
        } else {
            None
        };
        EmbedMetricsSnapshot {
            requests,
            texts: self.texts.load(Ordering::Relaxed),
            total_tokens: self.total_tokens.load(Ordering::Relaxed),
            avg_latency_ms,
        }
    }
}
