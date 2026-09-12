//! Storage errors.

use thiserror::Error;

/// A storage operation failed.
#[derive(Error, Debug)]
pub enum StorageError {
    /// No stored vector has the given id.
    #[error("Vector not found: {0}")]
    VectorNotFound(String),

    /// No collection has the given name.
    #[error("Collection not found: {0}")]
    CollectionNotFound(String),

    /// A collection with the given name already exists.
    #[error("Collection already exists: {0}")]
    CollectionExists(String),

    /// A vector's width differs from the collection's.
    #[error("Invalid vector dimension: expected {expected}, got {actual}")]
    InvalidDimension {
        /// Width the collection already holds.
        expected: usize,
        /// Width of the vector offered.
        actual: usize,
    },

    /// Vector values are not usable.
    #[error("Invalid vector data: {0}")]
    InvalidVectorData(String),

    /// A collection path cannot name a collection.
    #[error("Invalid collection path: {0}")]
    InvalidPath(String),

    /// Stored data does not decode or is inconsistent.
    #[error("Storage corruption detected: {0}")]
    CorruptedData(String),

    /// No space is left for the write.
    #[error("Storage full: {0}")]
    StorageFull(String),

    /// A sidecar file does not decode.
    #[error("Index file corrupted: {0}")]
    CorruptedIndex(String),

    /// Mapping a data file into memory failed.
    #[error("Memory map error: {0}")]
    MemoryMapError(String),

    /// A lock could not be acquired.
    #[error("Lock acquisition failed: {0}")]
    LockFailed(String),

    /// A write to storage did not complete.
    #[error("Write operation failed: {0}")]
    WriteFailed(String),

    /// A read from storage did not complete.
    #[error("Read operation failed: {0}")]
    ReadFailed(String),
}
