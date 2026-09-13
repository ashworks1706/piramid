//! Opens a device and holds its memory budget and streams for everything else to borrow.

use crate::gpu::budget::{BudgetSettings, DeviceBudget};
use crate::gpu::device::Device;
use crate::gpu::error::{GpuError, GpuResult};
use crate::gpu::stream::Stream;

/// Owns the process's opened [Device], its [DeviceBudget] and its [Stream]s.
#[derive(Debug)]
pub struct GpuManager {
    device: Device,
    budget: DeviceBudget,
    streams: Vec<Stream>,
}

impl GpuManager {
    /// Opens the device at an ordinal with a memory budget and a number of independent streams.
    pub fn open(ordinal: usize, budget: BudgetSettings, streams: usize) -> GpuResult<Self> {
        if streams == 0 {
            return Err(GpuError::Runtime(
                "at least one stream is required".to_string(),
            ));
        }
        let device = Device::open(ordinal)?;
        let budget = DeviceBudget::new(device.capabilities().total_memory_bytes, budget)?;
        let streams = (0..streams)
            .map(|_| Stream::new(&device))
            .collect::<GpuResult<Vec<_>>>()?;
        Ok(Self {
            device,
            budget,
            streams,
        })
    }

    /// The device this manager opened.
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// The memory budget every allocation on the device draws from.
    pub fn budget(&self) -> &DeviceBudget {
        &self.budget
    }

    /// The independent streams opened with the device, in order.
    pub fn streams(&self) -> &[Stream] {
        &self.streams
    }
}
