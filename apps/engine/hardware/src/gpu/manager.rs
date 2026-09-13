//! The GPU domain entry: opens devices and hands out the handles everything else borrows.

use crate::gpu::device::Device;
use crate::gpu::error::GpuResult;
use crate::gpu::stream::Stream;

/// Owns device acquisition for the process.
///
/// Callers go through this to get a [Device]. The resources it hands out, [Device],
/// [crate::gpu::DeviceBuffer] and [Stream], keep their own names.
#[derive(Debug)]
pub struct GpuManager {
    device: Device,
}

impl GpuManager {
    /// Open the device at an ordinal; errors when no GPU backend is compiled in or none is present.
    pub fn open(ordinal: usize) -> GpuResult<Self> {
        Ok(Self {
            device: Device::open(ordinal)?,
        })
    }

    /// The device this manager opened.
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Create a stream on the managed device.
    pub fn stream(&self) -> GpuResult<Stream> {
        Stream::new(&self.device)
    }
}
