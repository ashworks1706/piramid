//! Strategy registry: one file per strategy, one arm in [for_mode].

mod parallel;
mod scalar;
mod simd;

pub use parallel::ParallelStrategy;
pub use scalar::ScalarStrategy;
pub use simd::SimdStrategy;

use crate::compute::error::ComputeResult;
use crate::compute::kernels::DistanceKernels;
use crate::compute::mode::ExecutionMode;

static SCALAR: ScalarStrategy = ScalarStrategy;
static SIMD: SimdStrategy = SimdStrategy;
static PARALLEL: ParallelStrategy = ParallelStrategy;

/// Every strategy compiled into this build, available or not.
pub fn all() -> Vec<&'static dyn DistanceKernels> {
    vec![&SCALAR, &SIMD, &PARALLEL]
}

/// The strategy serving a mode, resolving Auto first; an unavailable strategy is an error.
pub fn for_mode(mode: ExecutionMode) -> ComputeResult<&'static dyn DistanceKernels> {
    let strategy: &'static dyn DistanceKernels = match mode.resolve() {
        ExecutionMode::Scalar | ExecutionMode::Auto => &SCALAR,
        ExecutionMode::Simd => &SIMD,
        ExecutionMode::Parallel => &PARALLEL,
        ExecutionMode::Gpu => {
            return Err(crate::compute::error::ComputeError::StrategyUnavailable {
                strategy: "gpu",
                reason: "no GPU distance kernels are implemented".to_string(),
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
