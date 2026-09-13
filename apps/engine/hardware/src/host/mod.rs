//! The host: processor and memory readings for the machine and for this process, and readings of
//! its GPUs.

#[cfg(feature = "gpu-cuda")]
mod nvml;
pub mod reading;
pub mod sampler;

pub use reading::{GpuReading, HostReading};
pub use sampler::{GpuSampler, HostSampler};
