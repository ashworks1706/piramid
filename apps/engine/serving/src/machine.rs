//! Readings of the machine the server runs on, refreshed on a background thread.

use std::sync::{Arc, Weak};
use std::time::Duration;

use parking_lot::RwLock;
use piramid_core::error::{Result, ServerError};
use piramid_hardware::host::{GpuReading, GpuSampler, HostReading, HostSampler};

/// Time between machine samples.
pub const SAMPLE_INTERVAL: Duration = Duration::from_secs(1);

/// The readings of one sample.
#[derive(Debug, Default)]
struct Latest {
    host: HostReading,
    gpus: Vec<GpuReading>,
}

/// The latest readings of the machine, updated by a sampling thread.
///
/// The thread stops after the last clone of this value is dropped.
#[derive(Debug, Clone)]
pub struct MachineReadings {
    latest: Arc<RwLock<Latest>>,
}

impl MachineReadings {
    /// Start a sampling thread that takes a reading every interval.
    ///
    /// Every field of the host reading is None, and there is no GPU reading, until the first
    /// sample lands. Errors when interval is below the host sampler minimum.
    pub fn start(interval: Duration) -> Result<Self> {
        if interval < HostSampler::MINIMUM_INTERVAL {
            return Err(ServerError::Internal(format!(
                "machine sample interval {interval:?} is below the minimum {:?}",
                HostSampler::MINIMUM_INTERVAL
            ))
            .into());
        }
        let latest = Arc::new(RwLock::new(Latest::default()));
        let slot = Arc::downgrade(&latest);
        std::thread::Builder::new()
            .name("piramid-machine".into())
            .spawn(move || sample_until_dropped(&slot, interval))
            .map_err(|e| ServerError::Internal(format!("start the machine sampler: {e}")))?;
        Ok(Self { latest })
    }

    /// The latest host reading.
    pub fn host(&self) -> HostReading {
        self.latest.read().host
    }

    /// The latest reading of each GPU. Empty when no GPU is measured.
    pub fn gpus(&self) -> Vec<GpuReading> {
        self.latest.read().gpus.clone()
    }
}

/// Samples the host and its GPUs into slot every interval until slot has no owner left.
fn sample_until_dropped(slot: &Weak<RwLock<Latest>>, interval: Duration) {
    let mut host_sampler = HostSampler::new();
    let mut gpu_sampler = GpuSampler::new();
    loop {
        let host = host_sampler.sample();
        let gpus = gpu_sampler.sample();
        let Some(latest) = slot.upgrade() else {
            return;
        };
        *latest.write() = Latest { host, gpus };
        drop(latest);
        std::thread::sleep(interval);
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a failed assertion is the point of a test"
)]
mod tests {
    use super::*;

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
        let machine =
            MachineReadings::start(HostSampler::MINIMUM_INTERVAL).expect("the thread starts");
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
}
