#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]
//! Machine readings and the sampling thread that refreshes them.

use std::sync::Arc;
use std::time::Duration;

use parking_lot::RwLock;
use piramid_hardware::host::{HostReading, HostSampler};
use piramid_serving::machine::{Latest, MachineReadings};

#[test]
fn a_reading_is_absent_until_the_first_sample() {
    let machine = MachineReadings {
        latest: Arc::new(RwLock::new(Latest::default())),
    };
    assert_eq!(machine.host(), HostReading::default());
    assert!(machine.gpus().is_empty());
}

#[test]
fn an_interval_below_the_minimum_is_refused() {
    let error = MachineReadings::start(HostSampler::MINIMUM_INTERVAL / 2)
        .expect_err("a short interval is refused");
    assert!(error.to_string().contains("below the minimum"), "{error}");
}

#[cfg(target_os = "linux")]
#[test]
fn the_sampling_thread_fills_in_processor_and_memory() {
    let machine = MachineReadings::start(HostSampler::MINIMUM_INTERVAL).expect("the thread starts");
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while machine.host().cpu_percent.is_none() && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    let host = machine.host();
    assert!(host.cpu_percent.is_some());
    assert!(host.process_cpu_percent.is_some());
    assert!(host.memory_total_bytes.is_some_and(|total| total > 0));
    assert!(host.process_resident_bytes.is_some());
}
