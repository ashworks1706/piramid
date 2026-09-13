//! Model execution errors.

use thiserror::Error;

/// A model could not be loaded or a generation could not be run.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum InferenceError {
    /// No model runtime is compiled in, no model is loaded, or the device is missing.
    #[error("inference unavailable: {0}")]
    Unavailable(String),

    /// Weights, tokenizer or model configuration could not be read or do not fit together.
    #[error("model load failed: {0}")]
    Load(String),

    /// The request is malformed or asks for more than the loaded model allows.
    #[error("invalid generation request: {0}")]
    InvalidRequest(String),

    /// The admission queue is full.
    #[error("generation queue is full: {0}")]
    QueueFull(String),

    /// A request waited longer than its queue timeout.
    #[error("generation timed out: {0}")]
    Timeout(String),

    /// The engine is shutting down or has stopped.
    #[error("inference engine stopped: {0}")]
    Stopped(String),

    /// The forward pass or sampler failed.
    #[error("inference runtime error: {0}")]
    Runtime(String),
}
