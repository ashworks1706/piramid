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
pub struct Latest {
    /// The host reading.
    pub host: HostReading,
    /// The reading of each GPU.
    pub gpus: Vec<GpuReading>,
}

/// The latest readings of the machine, updated by a sampling thread.
///
/// The thread stops after the last clone of this value is dropped.
#[derive(Debug, Clone)]
pub struct MachineReadings {
    /// The most recent sample, shared with the sampling thread.
    pub latest: Arc<RwLock<Latest>>,
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
