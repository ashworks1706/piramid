//! Error types for every layer, and the classification they share.

pub mod config;
pub mod context;
pub mod embedding;
pub mod index;
pub mod kind;
pub mod server;
pub mod storage;

pub use config::ConfigError;
pub use context::ErrorContext;
pub use embedding::EmbeddingError;
pub use index::IndexError;
pub use kind::{ErrorKind, PiramidError, Result};
pub use server::ServerError;
pub use storage::StorageError;
