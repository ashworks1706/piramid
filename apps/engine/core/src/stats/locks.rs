//! Recording lock waits against an optional tracker.

use std::time::Instant;

use super::LatencyTracker;

/// Record the time since start as a read-lock wait, when a tracker is given.
pub fn record_lock_read(tracker: Option<&LatencyTracker>, start: Instant) {
    if let Some(tracker) = tracker {
        tracker.record_lock_read(start.elapsed());
    }
}

/// Record the time since start as a write-lock wait, when a tracker is given.
pub fn record_lock_write(tracker: Option<&LatencyTracker>, start: Instant) {
    if let Some(tracker) = tracker {
        tracker.record_lock_write(start.elapsed());
    }
}
