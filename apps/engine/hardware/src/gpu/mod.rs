//! Device runtime: contexts, memory, streams, and compiled kernels, shared by compute and inference.

pub mod backends;
pub mod budget;
pub mod buffer;
pub mod device;
pub mod error;
pub mod kernels;
pub mod manager;
pub mod module;
pub mod stream;

pub use budget::{BudgetSettings, DeviceBudget, MemoryPool, PoolShares, PoolUsage, Reservation};
pub use buffer::{DeviceAllocation, DeviceBuffer, DeviceElement};
pub use device::{Device, DeviceCapabilities, DeviceRuntime};
pub use error::{GpuError, GpuResult};
pub use manager::GpuManager;
pub use module::{KernelArg, KernelModule, LaunchConfig};
pub use stream::{Stream, DEFAULT_STREAM, PER_THREAD_STREAM};
