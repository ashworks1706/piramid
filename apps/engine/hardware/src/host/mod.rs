//! The host: processor, memory and GPU readings for the machine and this process.

#[cfg(feature = "gpu-cuda")]
mod nvml;
pub mod reading;
pub mod sampler;

pub use reading::{GpuReading, HostReading};
pub use sampler::{GpuSampler, HostSampler};
