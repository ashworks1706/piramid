//! Execution-mode selection: [ExecutionMode] names which strategy runs a kernel.

use serde::{Deserialize, Serialize};

/// Which execution strategy runs a kernel. Auto resolves to a concrete strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    /// The SIMD strategy on x86_64 and aarch64, the scalar strategy on every other target.
    #[default]
    Auto,
    /// Portable scalar reference implementation.
    Scalar,
    /// Explicitly vectorized CPU path through the wide crate. Available on x86_64 and aarch64.
    Simd,
    /// Rayon-parallel CPU path: batch rows fanned across threads scored by the SIMD kernels.
    Parallel,
    /// GPU device execution.
    Gpu,
}

impl ExecutionMode {
    /// Resolve Auto into the strategy it names on this target; other modes pass through unchanged.
    pub fn resolve(&self) -> ExecutionMode {
        match self {
            ExecutionMode::Auto => {
                if cfg!(any(target_arch = "x86_64", target_arch = "aarch64")) {
                    ExecutionMode::Simd
                } else {
                    ExecutionMode::Scalar
                }
            }
            other => *other,
        }
    }

    /// Stable lowercase name, matching the serde representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            ExecutionMode::Auto => "auto",
            ExecutionMode::Scalar => "scalar",
            ExecutionMode::Simd => "simd",
            ExecutionMode::Parallel => "parallel",
            ExecutionMode::Gpu => "gpu",
        }
    }
}
