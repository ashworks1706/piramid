//! Collection and compaction request and response shapes.

use serde::{Deserialize, Serialize};

/// Summary of one collection.
#[derive(Serialize)]
pub struct CollectionInfo {
    /// Collection name.
    pub name: String,
    /// Number of stored documents.
    pub count: usize,
    /// Creation time of the collection, in seconds since the Unix epoch.
    pub created_at: Option<u64>,
    /// Time of the last manifest update, in seconds since the Unix epoch.
    pub updated_at: Option<u64>,
    /// Vector width of the collection. Null until the first vector is stored.
    pub dimensions: Option<usize>,
    /// The metric every search of the collection scores with, as the configuration names it.
    pub metric: String,
}

/// The collections currently loaded in memory.
#[derive(Serialize)]
pub struct CollectionsResponse {
    /// One summary per loaded collection.
    pub collections: Vec<CollectionInfo>,
}

/// Create a collection, or open it if it already exists.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateCollectionRequest {
    /// Collection name, validated against the collection naming rules.
    pub name: String,
}

/// Result of a compaction.
#[derive(Serialize)]
pub struct CompactResponse {
    /// Number of live documents written to the compacted record store.
    pub documents: usize,
    /// Size of the record store before compaction, in bytes.
    pub bytes_before: u64,
    /// Size of the record store after compaction, in bytes.
    pub bytes_after: u64,
    /// Duration of the compaction, in whole milliseconds.
    pub latency_ms: f32,
}

/// Result of deleting a collection.
#[derive(Serialize)]
pub struct DeleteCollectionResponse {
    /// True. Deleting a collection that is neither open nor on disk is a not found error.
    pub deleted: bool,
}

/// A bare count, for endpoints whose whole answer is a number.
#[derive(Serialize)]
pub struct CountResponse {
    /// Number of stored documents.
    pub count: usize,
}
