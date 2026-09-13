//! Collection, index, compaction and duplicate-scan request and response shapes.

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

/// Scan a collection for pairs of near-identical documents.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DuplicateRequest {
    /// Similarity metric: cosine, euclidean or dot. The metric the collection is indexed by when
    /// omitted.
    #[serde(default)]
    pub metric: Option<String>,
    /// Minimum score for a pair to be reported. 0.95 when omitted.
    #[serde(default = "default_dup_threshold")]
    pub threshold: f32,
    /// Maximum number of pairs returned, highest score first. All pairs when omitted.
    #[serde(default)]
    pub limit: Option<usize>,
    /// Neighbours examined per document, counting the document itself. 49 when omitted.
    #[serde(default)]
    pub k: Option<usize>,
    /// HNSW candidate-list width. The collection default when omitted.
    #[serde(default)]
    pub ef: Option<usize>,
    /// IVF partitions to scan. The collection default when omitted.
    #[serde(default)]
    pub nprobe: Option<usize>,
}

fn default_dup_threshold() -> f32 {
    0.95
}

/// Two near-identical documents and their similarity.
#[derive(Serialize)]
pub struct DuplicatePair {
    /// Id of the document that sorts first.
    pub id_a: String,
    /// Id of the document that sorts second.
    pub id_b: String,
    /// Similarity of the two documents under the requested metric.
    pub score: f32,
}

/// Near-duplicate pairs found in a collection.
#[derive(Serialize)]
pub struct DuplicateResponse {
    /// Pairs at or above the threshold, highest score first.
    pub pairs: Vec<DuplicatePair>,
}

/// Statistics of the vector index of a collection.
#[derive(Serialize)]
pub struct IndexStatsResponse {
    /// Index family: Flat, HNSW or IVF.
    pub index_type: String,
    /// Number of vectors in the index.
    pub total_vectors: usize,
    /// Approximate resident size of the index, in bytes.
    pub memory_usage_bytes: usize,
    /// Family-specific statistics, tagged by a type field.
    pub details: serde_json::Value,
}

/// Result of starting an index rebuild or running a compaction.
#[derive(Serialize)]
pub struct RebuildIndexResponse {
    /// True when the operation was accepted or completed.
    pub success: bool,
    /// Duration of a compaction, in whole milliseconds. Absent for an index rebuild.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f32>,
}

/// State of the most recent index rebuild of a collection.
#[derive(Serialize)]
pub struct RebuildIndexStatusResponse {
    /// One of running, completed or failed.
    pub status: String,
    /// Start time of the rebuild, in seconds since the Unix epoch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<u64>,
    /// End time of the rebuild, in seconds since the Unix epoch. Absent while running.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<u64>,
    /// Duration of the rebuild, in whole milliseconds. Absent while running.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elapsed_ms: Option<f32>,
    /// Error message of a failed rebuild. Absent unless the status is failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
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
