#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]
//! Generation counters.

use piramid_core::stats::InferenceMetrics;
use std::time::Duration;

#[test]
fn averages_are_absent_until_measured() {
    let metrics = InferenceMetrics::default();
    let empty = metrics.snapshot();
    assert_eq!(empty.avg_time_to_first_token_ms, None);
    assert_eq!(empty.decode_tokens_per_second, None);
    assert_eq!(empty.prefix_hit_rate, None);

    metrics.record_first_token(Duration::from_millis(40));
    metrics.record_step(2, 0, 2, Duration::from_millis(100));
    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.avg_time_to_first_token_ms, Some(40.0));
    assert_eq!(snapshot.decode_tokens_per_second, Some(20.0));
    assert_eq!(snapshot.last_batch_size, 2);
}

#[test]
fn step_time_is_split_between_prefill_and_decode_by_tokens() {
    let metrics = InferenceMetrics::default();
    metrics.record_step(2, 30, 10, Duration::from_millis(400));
    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.prefill_tokens_per_second, Some(100.0));
    assert_eq!(snapshot.decode_tokens_per_second, Some(100.0));
}

#[test]
fn a_step_too_large_for_u64_products_splits_without_overflow() {
    let metrics = InferenceMetrics::default();
    metrics.record_step(1, u64::MAX / 2, u64::MAX / 2, Duration::from_secs(1_000));
    let snapshot = metrics.snapshot();
    assert!(snapshot.prefill_tokens_per_second.is_some());
    assert!(snapshot.decode_tokens_per_second.is_some());
}
