//! Compiled kernel modules: [KernelModule] is a loaded image, [LaunchConfig] the geometry of one
//! launch, [KernelArg] one bound argument.

use crate::gpu::buffer::DeviceBuffer;
use crate::gpu::device::Device;
use crate::gpu::error::{GpuError, GpuResult};
use crate::gpu::stream::Stream;

/// Grid and block geometry for a single kernel launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LaunchConfig {
    /// Blocks per grid, in x, y and z.
    pub grid: (u32, u32, u32),
    /// Threads per block, in x, y and z.
    pub block: (u32, u32, u32),
    /// Dynamically allocated shared memory per block, in bytes.
    pub shared_memory_bytes: u32,
}

impl LaunchConfig {
    /// One-dimensional geometry covering n elements at the given threads per block. A zero block
    /// size, and a block count that does not fit the grid, are errors.
    pub fn for_elements(n: usize, block_size: u32) -> GpuResult<Self> {
        if block_size == 0 {
            return Err(GpuError::Launch("block size is zero".to_string()));
        }
        let blocks = u32::try_from(n.div_ceil(block_size as usize).max(1)).map_err(|e| {
            GpuError::Launch(format!(
                "{n} elements at block size {block_size} exceed the grid: {e}"
            ))
        })?;
        Ok(Self {
            grid: (blocks, 1, 1),
            block: (block_size, 1, 1),
            shared_memory_bytes: 0,
        })
    }
}

/// One argument bound to a kernel launch, in declaration order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KernelArg {
    /// A device pointer.
    Pointer(u64),
    /// A 32-bit unsigned integer.
    U32(u32),
    /// A 32-bit signed integer.
    I32(i32),
    /// A 64-bit unsigned integer.
    U64(u64),
    /// A 32-bit float.
    F32(f32),
}

impl KernelArg {
    /// The device pointer of a buffer.
    pub fn buffer<T>(buffer: &DeviceBuffer<T>) -> Self {
        Self::Pointer(buffer.handle().ptr)
    }
}

/// A compiled device module, loaded once and reused across launches.
#[derive(Debug)]
pub struct KernelModule {
    device: Device,
    name: &'static str,
    id: u64,
}

impl KernelModule {
    /// Compile kernel source and load the named functions from it.
    pub fn compile(
        device: &Device,
        name: &'static str,
        source: &str,
        functions: &[&'static str],
    ) -> GpuResult<Self> {
        let id = device.runtime().compile_module(source, functions)?;
        Ok(Self {
            device: device.clone(),
            name,
            id,
        })
    }

    /// Queue one launch of a function from this module on a stream.
    pub fn launch(
        &self,
        function: &'static str,
        config: LaunchConfig,
        stream: &Stream,
        args: &[KernelArg],
    ) -> GpuResult<()> {
        self.device
            .runtime()
            .launch(self.id, function, config, stream.id(), args)
    }

    /// Module name, as used in logs and error messages.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Device this module is loaded on.
    pub fn device(&self) -> &Device {
        &self.device
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "a failed assertion is the point of a test"
)]
mod tests {
    use super::*;

    #[test]
    fn a_launch_covers_every_element() {
        let config = LaunchConfig::for_elements(1000, 256).unwrap();
        assert_eq!(config.grid, (4, 1, 1));
        assert_eq!(config.block, (256, 1, 1));
    }

    #[test]
    fn a_zero_block_size_is_refused() {
        assert!(matches!(
            LaunchConfig::for_elements(10, 0),
            Err(GpuError::Launch(_))
        ));
    }

    #[test]
    fn a_block_count_beyond_the_grid_is_refused() {
        let elements = (u32::MAX as usize) * 2;
        assert!(matches!(
            LaunchConfig::for_elements(elements, 1),
            Err(GpuError::Launch(_))
        ));
    }
}
