//! Liveness and readiness responses.

use serde::Serialize;

/// Liveness of the process.
#[derive(Serialize)]
pub struct HealthResponse {
    /// Always ok.
    pub status: &'static str,
    /// Version of the running binary.
    pub version: &'static str,
}

/// Health of one collection, loaded or present on disk only.
#[derive(Serialize)]
pub struct CollectionHealth {
    /// Collection name.
    pub name: String,
    /// True when the collection is open in memory.
    pub loaded: bool,
    /// Number of stored documents. Absent when not loaded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
    /// Index family: Flat, HNSW or IVF. Absent when not loaded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index_type: Option<String>,
    /// Time of the last checkpoint since the collection was opened, in seconds since the Unix
    /// epoch. Absent when not loaded or not yet checkpointed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_checkpoint: Option<u64>,
    /// Seconds since the last checkpoint. Absent whenever last_checkpoint is absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkpoint_age_secs: Option<u64>,
    /// Size of the write-ahead log file, in bytes. Absent when not loaded or the file does not
    /// exist.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wal_size_bytes: Option<u64>,
    /// Schema version from the collection manifest. Absent when not loaded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_version: Option<u32>,
}

/// Readiness of the process and the health of every collection.
#[derive(Serialize)]
pub struct ReadyzResponse {
    /// Version of the running binary.
    pub version: String,
    /// Data directory the server stores collections in.
    pub data_dir: String,
    /// Number of collections listed, loaded and on disk only.
    pub total_collections: usize,
    /// Number of collections open in memory.
    pub loaded_collections: usize,
    /// Documents across all loaded collections.
    pub total_vectors: usize,
    /// Size of the filesystem holding the data directory, in bytes. Absent on non-Unix targets.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk_total_bytes: Option<u64>,
    /// Bytes available to the server on that filesystem. Absent on non-Unix targets.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk_available_bytes: Option<u64>,
    /// Loaded collections first, then collections present on disk only.
    pub collections: Vec<CollectionHealth>,
}
