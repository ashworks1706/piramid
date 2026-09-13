//! Vendor backend adapters; the only place a vendor SDK type may appear.

#[cfg(feature = "gpu-cuda")]
pub mod cudarc;

use crate::gpu::device::Device;
use crate::gpu::error::GpuResult;

/// Open the device at an ordinal for whichever backend this build enables.
pub fn open(ordinal: usize) -> GpuResult<Device> {
    #[cfg(feature = "gpu-cuda")]
    {
        cudarc::open(ordinal)
    }
    #[cfg(not(feature = "gpu-cuda"))]
    {
        Err(crate::gpu::error::GpuError::Unavailable(format!(
            "no GPU backend compiled in to open device {ordinal}; rebuild with the gpu-cuda feature"
        )))
    }
}
