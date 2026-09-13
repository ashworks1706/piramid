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
    #[error("Sidecar file corrupted: {0}")]
    CorruptedSidecar(String),

    /// A collection's manifest has schema version 1.
    #[error("Collection '{collection}' was written by Piramid 0.2 and must be re-ingested")]
    LegacyManifest {
        /// Name of the collection.
        collection: String,
    },

    /// A collection's manifest has a schema version this build does not read.
    #[error("Collection '{collection}' has manifest schema version {found}, which this build does not read")]
    UnsupportedManifest {
        /// Name of the collection.
        collection: String,
        /// Schema version the manifest was written with.
        found: u32,
    },

    /// A compaction was committed on disk and the open collection could not adopt its files.
    #[error("Collection '{collection}' refuses writes until it is opened again: a committed compaction could not be finished: {reason}")]
    CompactionUnfinished {
        /// Name of the collection.
        collection: String,
        /// The failure that stopped the compaction.
        reason: String,
    },

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
