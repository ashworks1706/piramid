#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]
//! Latency tracking.

use piramid_core::stats::latency::LatencyTracker;
use std::time::Duration;

#[test]
fn tracker_records_latencies() {
    let tracker = LatencyTracker::new();
    assert!(tracker.avg_insert_latency_ms().is_none());

    tracker.record_insert(Duration::from_millis(10));
    tracker.record_insert(Duration::from_millis(20));
    tracker.record_search(Duration::from_millis(5));

    let insert_avg = tracker.avg_insert_latency_ms().unwrap();
    assert!(insert_avg > 10.0 && insert_avg < 20.1);
    let search_avg = tracker.avg_search_latency_ms().unwrap();
    assert!(search_avg > 4.0 && search_avg < 6.1);
}
