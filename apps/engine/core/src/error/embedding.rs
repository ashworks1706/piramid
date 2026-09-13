//! Embedding provider errors.

use thiserror::Error;

/// An embedding provider call failed.
#[derive(Error, Debug)]
pub enum EmbeddingError {
    /// The request did not complete, or the client could not be built.
    #[error("HTTP request failed: {0}")]
    RequestFailed(String),

    /// The provider answered with an error status.
    #[error("API error: {0}")]
    ApiError(String),

    /// The response body could not be decoded or held no embeddings.
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// The provider configuration is not usable.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// The provider refused the request for exceeding its rate limit.
    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    /// The provider rejected the credentials.
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    /// The provider could not be reached.
    #[error("Provider unavailable: {0}")]
    ProviderUnavailable(String),

    /// The request exceeded its timeout.
    #[error("Timeout: {0}")]
    Timeout(String),

    /// The provider does not serve the requested model.
    #[error("Invalid model: {0}")]
    InvalidModel(String),

    /// The provider refused the input, and refuses it again on every retry.
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

impl EmbeddingError {
    /// Whether retrying the same request can succeed.
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::RequestFailed(_)
            | Self::ApiError(_)
            | Self::InvalidResponse(_)
            | Self::RateLimitExceeded
            | Self::ProviderUnavailable(_)
            | Self::Timeout(_) => true,
            Self::ConfigError(_)
            | Self::AuthenticationFailed(_)
            | Self::InvalidModel(_)
            | Self::InvalidInput(_) => false,
        }
    }
}
