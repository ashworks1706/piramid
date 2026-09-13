#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]
//! Embedding throughput counters.

use piramid_core::stats::EmbedMetrics;
use std::time::Duration;

#[test]
fn tokens_are_absent_until_a_provider_reports_them() {
    let metrics = EmbedMetrics::default();
    metrics.record(1, 1, None, Duration::from_millis(1));
    assert_eq!(metrics.snapshot().total_tokens, None);
    assert_eq!(metrics.snapshot().requests, 1);

    metrics.record(1, 2, Some(0), Duration::from_millis(1));
    assert_eq!(metrics.snapshot().total_tokens, Some(0));

    metrics.record(1, 1, Some(7), Duration::from_millis(1));
    metrics.record(1, 1, None, Duration::from_millis(1));
    assert_eq!(metrics.snapshot().total_tokens, Some(7));
}
