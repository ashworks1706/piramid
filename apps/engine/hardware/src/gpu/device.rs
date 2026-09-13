//! Device discovery, capabilities, and the runtime handle.

use std::sync::Arc;

use crate::gpu::buffer::DeviceAllocation;
use crate::gpu::error::GpuResult;
use crate::gpu::module::{KernelArg, LaunchConfig};

/// What a device can do, probed once when it is opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceCapabilities {
    /// Human-readable device name.
    pub name: String,
    /// Zero-based device ordinal.
    pub ordinal: usize,
    /// CUDA compute capability as a major and minor pair.
    pub compute_capability: (u32, u32),
    /// Total device memory in bytes.
    pub total_memory_bytes: u64,
}

/// The contract a device runtime satisfies, implemented per vendor backend under
/// [crate::gpu::backends]. Streams and modules are named by the identifiers the runtime hands out.
pub trait DeviceRuntime: Send + Sync + std::fmt::Debug {
    /// Backend name, such as cudarc.
    fn name(&self) -> &'static str;

    /// Capabilities of the selected device.
    fn capabilities(&self) -> &DeviceCapabilities;

    /// Free device memory in bytes.
    fn available_memory_bytes(&self) -> GpuResult<u64>;

    /// Block until all queued work on this device completes.
    fn synchronize(&self) -> GpuResult<()>;

    /// Allocate the given number of bytes of device memory.
    fn allocate(&self, size_bytes: usize) -> GpuResult<DeviceAllocation>;

    /// Release an allocation made by [DeviceRuntime::allocate].
    fn free(&self, allocation: &DeviceAllocation) -> GpuResult<()>;

    /// Copy host bytes into a device allocation, returning once the copy has completed.
    fn copy_to_device(&self, dst: &DeviceAllocation, src: &[u8], stream: u64) -> GpuResult<()>;

    /// Copy device bytes into a host slice, returning once the bytes are on the host.
    fn copy_to_host(&self, src: &DeviceAllocation, dst: &mut [u8], stream: u64) -> GpuResult<()>;

    /// Create an independent execution stream and return its identifier.
    fn create_stream(&self) -> GpuResult<u64>;

    /// Block until every operation queued on the stream has completed.
    fn synchronize_stream(&self, stream: u64) -> GpuResult<()>;

    /// Compile kernel source and load the named functions from it, returning a module identifier.
    fn compile_module(&self, source: &str, functions: &[&'static str]) -> GpuResult<u64>;

    /// Queue one launch of a loaded function on a stream.
    fn launch(
        &self,
        module: u64,
        function: &'static str,
        config: LaunchConfig,
        stream: u64,
        args: &[KernelArg],
    ) -> GpuResult<()>;
}

/// A cheap-to-clone handle to one compute device, shared by the retrieval and inference paths.
#[derive(Debug, Clone)]
pub struct Device {
    runtime: Arc<dyn DeviceRuntime>,
}

impl Device {
    /// Wrap a backend runtime in a shareable handle.
    pub fn new(runtime: Arc<dyn DeviceRuntime>) -> Self {
        Self { runtime }
    }

    /// Open the device at an ordinal; unavailable when no GPU backend is compiled in.
    pub fn open(ordinal: usize) -> GpuResult<Self> {
        crate::gpu::backends::open(ordinal)
    }

    /// Borrow the underlying runtime.
    pub fn runtime(&self) -> &Arc<dyn DeviceRuntime> {
        &self.runtime
    }

    /// Capabilities of this device.
    pub fn capabilities(&self) -> &DeviceCapabilities {
        self.runtime.capabilities()
    }

    /// Free device memory in bytes.
    pub fn available_memory_bytes(&self) -> GpuResult<u64> {
        self.runtime.available_memory_bytes()
    }

    /// Block until all queued work completes.
    pub fn synchronize(&self) -> GpuResult<()> {
        self.runtime.synchronize()
    }
}
