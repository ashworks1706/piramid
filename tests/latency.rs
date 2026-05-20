use piramid::metrics::latency::{time_operation, time_operation_sync, LatencyTracker};
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

#[tokio::test]
async fn time_operation_async_measures_duration() {
    let (result, duration) = time_operation(async {
        tokio::time::sleep(Duration::from_millis(5)).await;
        7
    })
    .await;
    assert_eq!(result, 7);
    assert!(duration.as_millis() >= 5);
}

#[test]
fn time_operation_sync_measures_duration() {
    let (result, duration) = time_operation_sync(|| {
        std::thread::sleep(Duration::from_millis(5));
        "ok"
    });
    assert_eq!(result, "ok");
    assert!(duration.as_millis() >= 5);
}
