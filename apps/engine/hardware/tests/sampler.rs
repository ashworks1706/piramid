//! Host and GPU readings taken by the samplers.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]

use piramid_hardware::host::sampler::share_of_host;
use piramid_hardware::host::{GpuSampler, HostReading, HostSampler};

#[test]
fn the_first_sample_carries_no_processor_reading() {
    let mut sampler = HostSampler::new();
    let reading = sampler.sample();
    assert_eq!(reading.cpu_percent, None);
    assert_eq!(reading.process_cpu_percent, None);
}

#[test]
fn a_second_sample_reads_processor_use_on_a_supported_system() {
    let mut sampler = HostSampler::new();
    sampler.sample();
    std::thread::sleep(HostSampler::MINIMUM_INTERVAL);
    let reading = sampler.sample();
    if !sysinfo::IS_SUPPORTED_SYSTEM {
        assert_eq!(reading, HostReading::default());
        return;
    }
    let cpu = reading.cpu_percent.unwrap_or(-1.0);
    assert!((0.0..=100.0).contains(&cpu), "cpu_percent {cpu}");
    let process = reading.process_cpu_percent.unwrap_or(-1.0);
    assert!(
        (0.0..=100.0).contains(&process),
        "process_cpu_percent {process}"
    );
}

#[test]
fn memory_is_read_from_the_first_sample_on_a_supported_system() {
    let reading = HostSampler::new().sample();
    if !sysinfo::IS_SUPPORTED_SYSTEM {
        assert_eq!(reading, HostReading::default());
        return;
    }
    let total = reading.memory_total_bytes.unwrap_or(0);
    let used = reading.memory_used_bytes.unwrap_or(u64::MAX);
    assert!(total > 0);
    assert!(used <= total);
    assert!(reading.process_resident_bytes.is_some_and(|rss| rss > 0));
}

#[cfg(not(feature = "gpu-cuda"))]
#[test]
fn a_build_without_the_gpu_feature_reads_no_gpu() {
    assert!(GpuSampler::new().sample().is_empty());
}

#[cfg(feature = "gpu-cuda")]
#[test]
#[ignore = "needs an NVIDIA driver and device"]
fn a_real_device_reports_memory_utilization_and_temperature() {
    let readings = GpuSampler::new().sample();
    assert!(!readings.is_empty(), "no device was read");
    for reading in &readings {
        let total = reading.memory_total_bytes.unwrap_or(0);
        let used = reading.memory_used_bytes.unwrap_or(u64::MAX);
        assert!(total > 0, "{reading:?}");
        assert!(used <= total, "{reading:?}");
        let busy = reading.utilization_percent.unwrap_or(-1.0);
        assert!((0.0..=100.0).contains(&busy), "{reading:?}");
        let celsius = reading.temperature_celsius.unwrap_or(-1.0);
        assert!((1.0..=150.0).contains(&celsius), "{reading:?}");
    }
}

#[test]
fn process_use_is_scaled_to_every_cpu_of_the_host() {
    assert!((share_of_host(200.0, 8) - 25.0).abs() < f32::EPSILON);
    assert!((share_of_host(50.0, 1) - 50.0).abs() < f32::EPSILON);
}

#[test]
fn process_use_is_scaled_by_a_cpu_count_above_the_u16_range() {
    assert!((share_of_host(700_000.0, 70_000) - 10.0).abs() < 1e-4);
}
