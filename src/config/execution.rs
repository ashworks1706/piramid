// Execution mode configuration for vector operations
// Allows users to choose between SIMD-accelerated and scalar implementations

use serde::{Deserialize, Serialize};

/// Execution mode for vector operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionMode {
    /// Automatically detect and use SIMD if available (default)
    Auto,
    /// Force SIMD usage (will panic if not supported)
    Simd,
    /// Use scalar (non-SIMD) implementation
    Scalar,
}

impl Default for ExecutionMode {
    fn default() -> Self {
        ExecutionMode::Auto
    }
}

impl ExecutionMode {
    /// Check if SIMD should be used
    pub fn should_use_simd(&self) -> bool {
        match self {
            ExecutionMode::Auto => {
                // In Auto mode, we default to SIMD since we use the `wide` crate
                // which provides portable SIMD. In a more advanced implementation,
                // we could detect CPU features here.
                true
            }
            ExecutionMode::Simd => true,
            ExecutionMode::Scalar => false,
        }
    }
}
