use crate::metrics::LatencyTracker;
use std::time::Instant;

/// Records lock wait time if a tracker exists.
pub fn record_lock_read(tracker: Option<&LatencyTracker>, start: Instant) {
    if let Some(t) = tracker {
        t.record_lock_read(start.elapsed());
    }
}

/// Records lock wait time if a tracker exists.
pub fn record_lock_write(tracker: Option<&LatencyTracker>, start: Instant) {
    if let Some(t) = tracker {
        t.record_lock_write(start.elapsed());
    }
}
