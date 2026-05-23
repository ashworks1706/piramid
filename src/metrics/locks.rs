use std::time::Instant;

use super::LatencyTracker;

pub fn record_lock_read(tracker: Option<&LatencyTracker>, start: Instant) {
    if let Some(tracker) = tracker {
        tracker.record_lock_read(start.elapsed());
    }
}

pub fn record_lock_write(tracker: Option<&LatencyTracker>, start: Instant) {
    if let Some(tracker) = tracker {
        tracker.record_lock_write(start.elapsed());
    }
}
