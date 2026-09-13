//! Errors raised by compute kernels.

use thiserror::Error;

/// A kernel failed to run: dimension mismatch, unavailable strategy, or a device fault.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ComputeError {
    /// A metric name that does not match any variant.
    #[error("unknown metric '{name}'; expected cosine, euclidean or dot")]
    UnknownMetric {
        /// The name that was asked for.
        name: String,
    },
    /// The requested strategy is not compiled in, or the hardware it needs is absent.
    #[error("compute strategy '{strategy}' unavailable: {reason}")]
    StrategyUnavailable {
        /// Strategy that was requested.
        strategy: &'static str,
        /// Why it could not be used.
        reason: String,
    },
    /// Operand shapes disagree.
    #[error("compute shape mismatch: expected {expected}, got {got}")]
    ShapeMismatch {
        /// What the kernel expected.
        expected: usize,
        /// What it received.
        got: usize,
    },
    /// A quantized vector's encoding cannot be decoded, or its level has no encoder.
    #[error("invalid quantized encoding: {0}")]
    InvalidEncoding(String),
    /// The underlying vendor strategy ran but failed.
    #[error("compute strategy '{strategy}' failed: {message}")]
    StrategyFailed {
        /// Vendor strategy that failed.
        strategy: &'static str,
        /// Underlying message.
        message: String,
    },
}

/// Convenience alias for kernel results.
pub type ComputeResult<T> = std::result::Result<T, ComputeError>;
