//! Execution streams: a [Stream] is an ordered queue of device work, and separate streams overlap.

use crate::gpu::device::Device;
use crate::gpu::error::GpuResult;

/// Identifier of the device default stream.
pub const DEFAULT_STREAM: u64 = 0;

/// Identifier of the per-thread stream, the queue a model runtime on the calling thread uses.
pub const PER_THREAD_STREAM: u64 = 1;

/// An ordered queue of device operations.
#[derive(Debug, Clone)]
pub struct Stream {
    device: Device,
    id: u64,
}

impl Stream {
    /// The default stream for the device.
    pub fn default_for(device: &Device) -> Self {
        Self {
            device: device.clone(),
            id: DEFAULT_STREAM,
        }
    }

    /// The per-thread stream for the device, shared with any runtime queueing from the same thread.
    pub fn per_thread(device: &Device) -> Self {
        Self {
            device: device.clone(),
            id: PER_THREAD_STREAM,
        }
    }

    /// Create an independent stream that can overlap with others.
    pub fn new(device: &Device) -> GpuResult<Self> {
        let id = device.runtime().create_stream()?;
        Ok(Self {
            device: device.clone(),
            id,
        })
    }

    /// Backend stream identifier.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Device this stream belongs to.
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Block until every operation queued on this stream has completed.
    pub fn synchronize(&self) -> GpuResult<()> {
        self.device.runtime().synchronize_stream(self.id)
    }
}
