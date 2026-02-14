// Metrics tracking for embedding requests, including counts and latency.
// This module defines the `EmbedMetrics` struct, which uses atomic counters to track the number of embedding requests, total texts embedded, total tokens processed, and total latency. It also provides a method to take a snapshot of the current metrics for reporting purposes.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

#[derive(Default)]
pub struct EmbedMetrics {
    requests: AtomicU64, // Total number of embedding requests made
    texts: AtomicU64, // Total number of texts embedded (sum of input texts across all requests)
    total_tokens: AtomicU64, // Total number of tokens processed (if available from the embedding provider)
    total_latency_ns: AtomicU64, // Total latency in nanoseconds across all embedding requests
}

#[derive(Debug, Clone, Copy)]
pub struct EmbedMetricsSnapshot {
    pub requests: u64, // Total number of embedding requests made
    pub texts: u64, // Total number of texts embedded
    pub total_tokens: u64, // Total number of tokens processed
    pub avg_latency_ms: Option<f32>, // Average latency in milliseconds per request (if requests > 0)
}

impl EmbedMetrics {

    pub fn record(&self, request_count: u64, text_count: u64, token_count: u64, latency: Duration) {
        self.requests.fetch_add(request_count, Ordering::Relaxed);
        self.texts.fetch_add(text_count, Ordering::Relaxed);
        self.total_tokens.fetch_add(token_count, Ordering::Relaxed);
        self.total_latency_ns
            .fetch_add(latency.as_nanos() as u64, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> EmbedMetricsSnapshot {
        let requests = self.requests.load(Ordering::Relaxed);
        let total_latency_ns = self.total_latency_ns.load(Ordering::Relaxed); // Total latency in nanoseconds
        let avg_latency_ms = if requests > 0 {
            Some((total_latency_ns as f64 / requests as f64 / 1_000_000.0) as f32) // Convert to milliseconds
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
