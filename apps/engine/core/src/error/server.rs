//! Request-level errors, each mapped to an [ErrorKind].

use thiserror::Error;

use super::kind::ErrorKind;

/// A request could not be served.
#[derive(Error, Debug)]
pub enum ServerError {
    /// The request is malformed.
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// The request is well-formed but a value failed validation.
    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    /// The named resource does not exist.
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// The named resource already exists.
    #[error("Resource already exists: {0}")]
    AlreadyExists(String),

    /// Credentials were missing or invalid.
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    /// Credentials were valid but do not permit the operation.
    #[error("Authorization failed: {0}")]
    AuthorizationFailed(String),

    /// The caller exceeded a rate limit.
    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    /// The request did not finish in time.
    #[error("Request timeout")]
    Timeout,

    /// An unexpected failure inside the server.
    #[error("Internal server error: {0}")]
    Internal(String),

    /// The server cannot serve this request right now.
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
}

impl ServerError {
    /// Transport-agnostic classification for this error.
    pub fn kind(&self) -> ErrorKind {
        match self {
            Self::InvalidRequest(_) | Self::ValidationFailed(_) => ErrorKind::BadRequest,
            Self::NotFound(_) => ErrorKind::NotFound,
            Self::AlreadyExists(_) => ErrorKind::Conflict,
            Self::AuthenticationFailed(_) => ErrorKind::Unauthenticated,
            Self::AuthorizationFailed(_) => ErrorKind::Forbidden,
            Self::RateLimitExceeded => ErrorKind::RateLimited,
            Self::Timeout => ErrorKind::Timeout,
            Self::Internal(_) => ErrorKind::Internal,
            Self::ServiceUnavailable(_) => ErrorKind::Unavailable,
        }
    }
}
