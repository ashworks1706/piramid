// Execution mode configuration for vector operations

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ExecutionMode {
    #[default]
    Auto,
    Simd,
    Scalar,
    Gpu,
    Parallel,
    Binary,
    Jit,
}

impl ExecutionMode {
    pub fn resolve(&self) -> ExecutionMode {
        match self {
            ExecutionMode::Auto => {
                // Auto-detect best execution mode based on CPU features
                #[cfg(target_arch = "x86_64")]
                {
                    if is_x86_feature_detected!("avx2") {
                        ExecutionMode::Simd
                    } else {
                        ExecutionMode::Scalar
                    }
                }
                
                #[cfg(target_arch = "aarch64")]
                {
                    if std::arch::is_aarch64_feature_detected!("neon") {
                        ExecutionMode::Simd
                    } else {
                        ExecutionMode::Scalar
                    }
                }
                
                #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
                {
                    ExecutionMode::Scalar
                }
            },
            ExecutionMode::Simd => {
                #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
                {
                    ExecutionMode::Simd
                }
                #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
                {
                    ExecutionMode::Scalar
                }
            },
            ExecutionMode::Scalar => ExecutionMode::Scalar,
            ExecutionMode::Gpu => {
                // GPU not implemented, fallback to best available
                ExecutionMode::Auto.resolve()
            },
            ExecutionMode::Parallel => ExecutionMode::Parallel,
            ExecutionMode::Binary => ExecutionMode::Binary,
            ExecutionMode::Jit => ExecutionMode::Jit,
        }
    }
    
    pub fn use_simd(&self) -> bool {
        matches!(self.resolve(), ExecutionMode::Simd | ExecutionMode::Parallel)
    }
    
    pub fn use_parallel(&self) -> bool {
        matches!(self.resolve(), ExecutionMode::Parallel)
    }
}
