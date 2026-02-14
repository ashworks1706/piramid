use thiserror::Error;

// Define the error type for embedding operations. This enum represents various kinds of errors that can occur when working with embedding providers, such as HTTP request failures, API errors, invalid responses, configuration issues, rate limits, authentication failures, provider unavailability, timeouts, and invalid models. Each variant includes a message that provides more details about the error. The is_recoverable method allows us to determine if an error is something that we can retry or if it is a fatal error that should not be retried.
#[derive(Error, Debug)]
pub enum EmbeddingError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Provider unavailable: {0}")]
    ProviderUnavailable(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Invalid model: {0}")]
    InvalidModel(String),
}

impl EmbeddingError {
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::RequestFailed(_) => true,
            Self::ApiError(_) => true,
            Self::InvalidResponse(_) => true,
            Self::ConfigError(_) => false,
            Self::RateLimitExceeded => true,
            Self::AuthenticationFailed(_) => false,
            Self::ProviderUnavailable(_) => true,
            Self::Timeout(_) => true,
            Self::InvalidModel(_) => false,
        }
    }
}
