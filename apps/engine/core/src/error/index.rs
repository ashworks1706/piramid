//! Index errors.

use thiserror::Error;

/// An index operation failed.
#[derive(Error, Debug)]
pub enum IndexError {
    /// The index has not been built.
    #[error("Index not initialized")]
    NotInitialized,

    /// The index structure is inconsistent.
    #[error("Index corrupted: {0}")]
    Corrupted(String),

    /// The index parameters are not usable.
    #[error("Invalid index configuration: {0}")]
    InvalidConfig(String),

    /// Building the index did not complete.
    #[error("Index build failed: {0}")]
    BuildFailed(String),

    /// A search over the index did not complete.
    #[error("Search failed: {0}")]
    SearchFailed(String),

    /// A graph node the index refers to is missing.
    #[error("Node not found: {0}")]
    NodeNotFound(String),

    /// A graph layer outside the index's range was addressed.
    #[error("Invalid layer: {0}")]
    InvalidLayer(String),

    /// Writing the index to disk failed.
    #[error("Index persistence failed: {0}")]
    PersistenceFailed(String),

    /// Reading the index from disk failed.
    #[error("Index load failed: {0}")]
    LoadFailed(String),
}
