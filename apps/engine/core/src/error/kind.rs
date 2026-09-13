//! The top-level error and its transport-agnostic classification.

use std::io;
use thiserror::Error;

use super::inference::InferenceError;

/// Result with [PiramidError] as the error.
pub type Result<T> = std::result::Result<T, PiramidError>;

/// What kind of failure occurred, independent of any wire protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// The request was malformed or failed validation.
    BadRequest,
    /// The requested resource does not exist.
    NotFound,
    /// The resource already exists.
    Conflict,
    /// Credentials were missing or invalid.
    Unauthenticated,
    /// Credentials were valid but insufficient.
    Forbidden,
    /// The caller exceeded a rate limit.
    RateLimited,
    /// The operation timed out.
    Timeout,
    /// A dependency the server calls out to failed.
    Upstream,
    /// The server is temporarily unable to serve.
    Unavailable,
    /// An unexpected internal failure.
    Internal,
}

/// Any error the engine returns.
#[derive(Error, Debug)]
pub enum PiramidError {
    /// A storage failure.
    #[error("Storage error: {0}")]
    Storage(#[from] super::storage::StorageError),

    /// A search failure.
    #[error("Search error: {0}")]
    Search(#[from] super::search::SearchError),

    /// A configuration that cannot be applied.
    #[error("Configuration error: {0}")]
    Config(#[from] super::config::ConfigError),

    /// A request-level failure with its own classification.
    #[error("Server error: {0}")]
    Server(#[from] super::server::ServerError),

    /// An embedding provider failure.
    #[error("Embedding error: {0}")]
    Embedding(#[from] super::embedding::EmbeddingError),

    /// A model load or generation failure.
    #[error("Inference error: {0}")]
    Inference(#[from] InferenceError),

    /// A distance kernel or strategy failure.
    #[error("Compute error: {0}")]
    Compute(#[from] piramid_hardware::compute::ComputeError),

    /// A GPU device failure.
    #[error("Device error: {0}")]
    Gpu(#[from] piramid_hardware::gpu::GpuError),

    /// A filesystem or other I/O failure.
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// A bincode encode failure.
    #[error("Encoding error: {0}")]
    Encode(#[from] bincode::error::EncodeError),

    /// A bincode decode failure.
    #[error("Decoding error: {0}")]
    Decode(#[from] bincode::error::DecodeError),

    /// A JSON encode or decode failure.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Any other failure, described by its message.
    #[error("{0}")]
    Other(String),
}

impl PiramidError {
    /// A [PiramidError::Other] carrying msg.
    pub fn other<S: Into<String>>(msg: S) -> Self {
        Self::Other(msg.into())
    }

    /// Transport-agnostic classification of this error.
    pub fn kind(&self) -> ErrorKind {
        match self {
            Self::Server(e) => e.kind(),
            Self::Embedding(_) => ErrorKind::Upstream,
            Self::Inference(e) => match e {
                InferenceError::InvalidRequest(_) => ErrorKind::BadRequest,
                InferenceError::Timeout(_) => ErrorKind::Timeout,
                InferenceError::Unavailable(_)
                | InferenceError::QueueFull(_)
                | InferenceError::Stopped(_) => ErrorKind::Unavailable,
                InferenceError::Load(_) | InferenceError::Runtime(_) => ErrorKind::Internal,
            },
            Self::Search(super::search::SearchError::MetricMismatch { .. })
            | Self::Config(_)
            | Self::Storage(
                super::storage::StorageError::InvalidDimension { .. }
                | super::storage::StorageError::InvalidPath(_),
            ) => ErrorKind::BadRequest,
            Self::Compute(piramid_hardware::compute::ComputeError::UnknownMetric { .. }) => {
                ErrorKind::BadRequest
            }
            Self::Compute(piramid_hardware::compute::ComputeError::StrategyUnavailable {
                ..
            })
            | Self::Gpu(_) => ErrorKind::Unavailable,
            Self::Compute(_)
            | Self::Storage(_)
            | Self::Io(_)
            | Self::Encode(_)
            | Self::Decode(_)
            | Self::Json(_)
            | Self::Other(_) => ErrorKind::Internal,
        }
    }
}
