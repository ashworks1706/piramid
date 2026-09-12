//! Readings of the machine the server runs on, refreshed on a background thread.

use std::sync::{Arc, Weak};
use std::time::Duration;

use parking_lot::RwLock;
use piramid_core::error::{Result, ServerError};
use piramid_hardware::host::{HostReading, HostSampler};

/// Time between machine samples.
pub const SAMPLE_INTERVAL: Duration = Duration::from_secs(1);

/// The latest readings of the machine, updated by a sampling thread.
///
/// The thread stops after the last clone of this value is dropped.
#[derive(Debug, Clone)]
pub struct MachineReadings {
    host: Arc<RwLock<HostReading>>,
}

impl MachineReadings {
    /// Start a sampling thread that takes a reading every interval.
    ///
    /// Every field of the host reading is None until the first sample lands.
    pub fn start(interval: Duration) -> Result<Self> {
        let host = Arc::new(RwLock::new(HostReading::default()));
        let slot = Arc::downgrade(&host);
        let interval = interval.max(HostSampler::MINIMUM_INTERVAL);
        std::thread::Builder::new()
            .name("piramid-machine".into())
            .spawn(move || sample_until_dropped(&slot, interval))
            .map_err(|e| ServerError::Internal(format!("start the machine sampler: {e}")))?;
        Ok(Self { host })
    }

    /// The latest host reading.
    pub fn host(&self) -> HostReading {
        *self.host.read()
    }
}

/// Samples the host into slot every interval until slot has no owner left.
fn sample_until_dropped(slot: &Weak<RwLock<HostReading>>, interval: Duration) {
    let mut sampler = HostSampler::new();
    loop {
        let reading = sampler.sample();
        let Some(host) = slot.upgrade() else {
            return;
        };
        *host.write() = reading;
        drop(host);
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
            host: Arc::new(RwLock::new(HostReading::default())),
        };
        assert_eq!(machine.host(), HostReading::default());
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
