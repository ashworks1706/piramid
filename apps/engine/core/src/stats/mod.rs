//! What the engine measures about itself: latency, lock contention, embedding throughput,
//! generation.

pub mod embed;
pub mod inference;
pub mod latency;
pub mod locks;

pub use embed::{EmbedMetrics, EmbedMetricsSnapshot};
pub use inference::{EngineGauges, InferenceMetrics, InferenceMetricsSnapshot};
pub use latency::LatencyTracker;
pub use locks::{record_lock_read, record_lock_write};
