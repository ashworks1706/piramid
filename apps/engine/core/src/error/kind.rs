use std::io;
use thiserror::Error;

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

#[derive(Error, Debug)]
pub enum PiramidError {
    #[error("Storage error: {0}")]
    Storage(#[from] super::storage::StorageError),

    #[error("Index error: {0}")]
    Index(#[from] super::index::IndexError),

    #[error("Server error: {0}")]
    Server(#[from] super::server::ServerError),

    #[error("Embedding error: {0}")]
    Embedding(#[from] super::embedding::EmbeddingError),

    #[error("Compute error: {0}")]
    Compute(#[from] piramid_hardware::compute::ComputeError),
    #[error("Device error: {0}")]
    Gpu(#[from] piramid_hardware::gpu::GpuError),

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("{0}")]
    Other(String),
}

impl PiramidError {
    pub fn other<S: Into<String>>(msg: S) -> Self {
        Self::Other(msg.into())
    }

    /// Transport-agnostic classification of this error.
    pub fn kind(&self) -> ErrorKind {
        match self {
            Self::Server(e) => e.kind(),
            Self::Embedding(_) => ErrorKind::Upstream,
            Self::Index(super::index::IndexError::MetricMismatch { .. }) => ErrorKind::BadRequest,
            Self::Compute(piramid_hardware::compute::ComputeError::UnknownMetric { .. }) => {
                ErrorKind::BadRequest
            }
            Self::Compute(piramid_hardware::compute::ComputeError::StrategyUnavailable {
                ..
            })
            | Self::Gpu(_) => ErrorKind::Unavailable,
            Self::Compute(_)
            | Self::Storage(_)
            | Self::Index(_)
            | Self::Io(_)
            | Self::Serialization(_)
            | Self::Json(_)
            | Self::Other(_) => ErrorKind::Internal,
        }
    }
}
