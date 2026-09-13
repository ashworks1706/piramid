//! Strategy registry: one file per strategy, one arm in [for_mode].

#[cfg(feature = "gpu-cuda")]
mod cuda;
mod parallel;
mod scalar;
mod simd;

#[cfg(feature = "gpu-cuda")]
pub use cuda::{install_gpu, CudaStrategy};
pub use parallel::ParallelStrategy;
pub use scalar::ScalarStrategy;
pub use simd::SimdStrategy;

use crate::compute::error::ComputeResult;
use crate::compute::kernels::DistanceKernels;
use crate::compute::mode::ExecutionMode;

static SCALAR: ScalarStrategy = ScalarStrategy;
static SIMD: SimdStrategy = SimdStrategy;
static PARALLEL: ParallelStrategy = ParallelStrategy;
#[cfg(feature = "gpu-cuda")]
static CUDA: CudaStrategy = CudaStrategy;

/// Every strategy compiled into this build, available or not.
pub fn all() -> Vec<&'static dyn DistanceKernels> {
    vec![
        &SCALAR,
        &SIMD,
        &PARALLEL,
        #[cfg(feature = "gpu-cuda")]
        &CUDA,
    ]
}

/// Whether this build can serve a mode at all: a CPU mode on its target, or the gpu mode with a
/// GPU backend compiled in. A gpu mode still needs a device installed before [for_mode] serves it.
pub fn compiled(mode: ExecutionMode) -> ComputeResult<()> {
    match mode.resolve() {
        ExecutionMode::Gpu => {
            if cfg!(feature = "gpu-cuda") {
                Ok(())
            } else {
                Err(crate::compute::error::ComputeError::StrategyUnavailable {
                    strategy: "gpu",
                    reason: "no GPU backend compiled in; rebuild with the gpu-cuda feature"
                        .to_string(),
                })
            }
        }
        _ => for_mode(mode).map(|_| ()),
    }
}

/// The strategy serving a mode, resolving Auto first; an unavailable strategy is an error.
pub fn for_mode(mode: ExecutionMode) -> ComputeResult<&'static dyn DistanceKernels> {
    let strategy: &'static dyn DistanceKernels = match mode.resolve() {
        ExecutionMode::Scalar | ExecutionMode::Auto => &SCALAR,
        ExecutionMode::Simd => &SIMD,
        ExecutionMode::Parallel => &PARALLEL,
        #[cfg(feature = "gpu-cuda")]
        ExecutionMode::Gpu => &CUDA,
        #[cfg(not(feature = "gpu-cuda"))]
        ExecutionMode::Gpu => {
            return Err(crate::compute::error::ComputeError::StrategyUnavailable {
                strategy: "gpu",
                reason: "no GPU backend compiled in; rebuild with the gpu-cuda feature".to_string(),
            });
        }
    };

    if strategy.is_available() {
        Ok(strategy)
    } else {
        Err(crate::compute::error::ComputeError::StrategyUnavailable {
            strategy: strategy.name(),
            reason: "not available on this machine".to_string(),
        })
    }
}
