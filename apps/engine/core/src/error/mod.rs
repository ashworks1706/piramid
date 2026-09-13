//! Error types for every layer, and the classification they share.

pub mod config;
pub mod context;
pub mod embedding;
pub mod inference;
pub mod kind;
pub mod search;
pub mod server;
pub mod storage;

pub use config::ConfigError;
pub use context::ErrorContext;
pub use embedding::EmbeddingError;
pub use inference::InferenceError;
pub use kind::{ErrorKind, PiramidError, Result};
pub use search::SearchError;
pub use server::ServerError;
pub use storage::StorageError;
