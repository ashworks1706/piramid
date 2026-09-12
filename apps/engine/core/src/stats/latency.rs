//! Moving averages of operation latency, per collection.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Moving-average latencies for writes, searches and lock waits. Clones share the same counters.
#[derive(Debug, Clone, Default)]
pub struct LatencyTracker {
    // Microseconds, held as integers.
    insert_latency_us: Arc<AtomicU64>,
    search_latency_us: Arc<AtomicU64>,
    delete_latency_us: Arc<AtomicU64>,
    update_latency_us: Arc<AtomicU64>,
    lock_read_latency_us: Arc<AtomicU64>,
    lock_write_latency_us: Arc<AtomicU64>,

    insert_count: Arc<AtomicU64>,
    search_count: Arc<AtomicU64>,
    delete_count: Arc<AtomicU64>,
    update_count: Arc<AtomicU64>,
    lock_read_count: Arc<AtomicU64>,
    lock_write_count: Arc<AtomicU64>,
}

impl LatencyTracker {
    /// A tracker with no samples.
    pub fn new() -> Self {
        Self::default()
    }

    /// Fold an insert duration into its average.
    pub fn record_insert(&self, duration: Duration) {
        self.insert_count.fetch_add(1, Ordering::Relaxed);
        let us = duration.as_micros() as u64;
        self.update_moving_average(&self.insert_latency_us, us, &self.insert_count);
    }

    /// Fold a search duration into its average.
    pub fn record_search(&self, duration: Duration) {
        self.search_count.fetch_add(1, Ordering::Relaxed);
        let us = duration.as_micros() as u64;
        self.update_moving_average(&self.search_latency_us, us, &self.search_count);
    }

    /// Fold a delete duration into its average.
    pub fn record_delete(&self, duration: Duration) {
        self.delete_count.fetch_add(1, Ordering::Relaxed);
        let us = duration.as_micros() as u64;
        self.update_moving_average(&self.delete_latency_us, us, &self.delete_count);
    }

    /// Fold an update duration into its average.
    pub fn record_update(&self, duration: Duration) {
        self.update_count.fetch_add(1, Ordering::Relaxed);
        let us = duration.as_micros() as u64;
        self.update_moving_average(&self.update_latency_us, us, &self.update_count);
    }

    /// Fold a read-lock wait into its average.
    pub fn record_lock_read(&self, duration: Duration) {
        self.lock_read_count.fetch_add(1, Ordering::Relaxed);
        let us = duration.as_micros() as u64;
        self.update_moving_average(&self.lock_read_latency_us, us, &self.lock_read_count);
    }

    /// Fold a write-lock wait into its average.
    pub fn record_lock_write(&self, duration: Duration) {
        self.lock_write_count.fetch_add(1, Ordering::Relaxed);
        let us = duration.as_micros() as u64;
        self.update_moving_average(&self.lock_write_latency_us, us, &self.lock_write_count);
    }

    /// Average insert latency in milliseconds. None while the average is zero.
    pub fn avg_insert_latency_ms(&self) -> Option<f32> {
        Self::avg_ms(&self.insert_latency_us)
    }

    /// Average search latency in milliseconds. None while the average is zero.
    pub fn avg_search_latency_ms(&self) -> Option<f32> {
        Self::avg_ms(&self.search_latency_us)
    }

    /// Average read-lock wait in milliseconds. None while the average is zero.
    pub fn avg_lock_read_latency_ms(&self) -> Option<f32> {
        Self::avg_ms(&self.lock_read_latency_us)
    }

    /// Average write-lock wait in milliseconds. None while the average is zero.
    pub fn avg_lock_write_latency_ms(&self) -> Option<f32> {
        Self::avg_ms(&self.lock_write_latency_us)
    }

    /// None until at least one sample has landed.
    fn avg_ms(latency_us: &AtomicU64) -> Option<f32> {
        let us = latency_us.load(Ordering::Relaxed);
        (us > 0).then(|| us as f32 / 1000.0)
    }

    /// Fold a new sample into the running average.
    fn update_moving_average(&self, avg: &AtomicU64, new_value: u64, count: &AtomicU64) {
        let current = avg.load(Ordering::Relaxed);
        let cnt = count.load(Ordering::Relaxed);

        // A plain mean for the first five samples, then an exponential moving average.
        if cnt <= 5 {
            let new_avg = ((current * (cnt - 1)) + new_value) / cnt;
            avg.store(new_avg, Ordering::Relaxed);
        } else {
            let new_avg = ((current * 4) + new_value) / 5;
            avg.store(new_avg, Ordering::Relaxed);
        }
    }
}
