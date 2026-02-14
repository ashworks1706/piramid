// These structs define the JSON shape for API communication.
// Serde does the heavy lifting: Serialize = Rust → JSON, Deserialize = JSON → Rust.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// HEALTH
// =============================================================================

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,  // &'static = string literal, lives forever
    pub version: &'static str, // &'static = string literal, lives forever
}

// =============================================================================
// COLLECTIONS
// =============================================================================

#[derive(Serialize)]
pub struct CollectionInfo {
    pub name: String, // Name of the collection
    pub count: usize, // Number of vectors in the collection
    pub created_at: Option<u64>, // Timestamp when the collection was created (in seconds since UNIX epoch)
    pub updated_at: Option<u64>, // Timestamp when the collection was last updated (in seconds since UNIX epoch)
    pub dimensions: Option<usize>, // Number of dimensions for vectors in this collection, if known
}

#[derive(Serialize)]
pub struct CollectionsResponse {
    pub collections: Vec<CollectionInfo>, // List of collections with their info
}

#[derive(Deserialize)]
pub struct CreateCollectionRequest {
    pub name: String, // Name of the collection to create
}

// =============================================================================
// VECTORS
// =============================================================================

// What the client sends to store a vector
#[derive(Deserialize)]
pub struct InsertRequest {
    #[serde(default)]
    pub vector: Option<Vec<f32>>, // Optional vector to store; if not provided, embedding will be generated from text
    #[serde(default)]
    pub vectors: Option<Vec<Vec<f32>>>, // Optional list of vectors for batch insert; if not provided, embeddings will be generated from texts
    #[serde(default)]
    pub text: Option<String>, // Optional text to associate with the vector; used for embedding generation if vector is not provided
    #[serde(default)]
    pub texts: Option<Vec<String>>, // Optional list of texts for batch insert; used for embedding generation if vectors are not provided
    #[serde(default)]  // single metadata map (used for single insert)
    pub metadata: HashMap<String, serde_json::Value>,
    #[serde(default)]  // per-item metadata for batch
    pub metadata_list: Vec<HashMap<String, serde_json::Value>>,
    #[serde(default)]  // if missing, defaults to false
    pub normalize: bool,  // Whether to normalize the vector(s) to unit length
}

// What we return after storing (single)
#[derive(Serialize)]
pub struct InsertResponse {
    pub id: String, // ID of the inserted vector (UUID string)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f32>, // Optional latency for the insert operation in milliseconds
}

#[derive(Serialize)]
pub struct MultiInsertResponse {
    pub ids: Vec<String>, // List of IDs for the inserted vectors (UUID strings)
    pub count: usize, // Number of vectors inserted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f32>, // Optional latency for the batch insert operation in milliseconds
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum InsertResultsResponse {
    Single(InsertResponse), // Response for single vector insert
    Multi(MultiInsertResponse), // Response for batch vector insert
}

// Full vector data returned to client
#[derive(Serialize)]
pub struct VectorResponse {
    pub id: String,
    pub vector: Vec<f32>,
    pub text: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

// Query params for listing vectors: ?limit=100&offset=0
#[derive(Deserialize)]
pub struct ListVectorsQuery {
    #[serde(default = "default_limit")]
    pub limit: usize, // How many vectors to return (default 100)
    #[serde(default)]
    pub offset: usize, // How many vectors to skip for pagination (default 0)
}

fn default_limit() -> usize { 100 }

// =============================================================================
// SEARCH
// =============================================================================

#[derive(Deserialize)]
pub struct SearchRequest {
    #[serde(default)]
    pub vector: Option<Vec<f32>>,
    #[serde(default)]
    pub vectors: Option<Vec<Vec<f32>>>,
    #[serde(default = "default_k")]
    pub k: usize,  // how many results to return
    #[serde(default)]
    pub metric: Option<String>,  // "cosine", "euclidean", "dot"
    #[serde(default)]
    pub ef: Option<usize>,
    #[serde(default)]
    pub nprobe: Option<usize>,
    #[serde(default)]
    pub overfetch: Option<usize>,
    #[serde(default)]
    pub preset: Option<String>, // "fast", "balanced", "high"
}

fn default_k() -> usize { 10 }

#[derive(Serialize)]
pub struct HitResponse {
    pub id: String,
    pub score: f32, // Similarity score (higher is more similar)
    pub text: String,
    pub metadata: HashMap<String, serde_json::Value>, // Metadata associated with the vector
}

#[derive(Serialize)]
pub struct SearchResponse {
    pub results: Vec<HitResponse>, 
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f32>,
}

#[derive(Serialize)]
pub struct MultiSearchResponse {
    pub results: Vec<Vec<HitResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f32>,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum SearchResultsResponse {
    Single(SearchResponse),
    Multi(MultiSearchResponse),
}

// =============================================================================
// COMMON
// =============================================================================

#[derive(Serialize)]
pub struct DeleteResponse {
    pub deleted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f32>,
}

#[derive(Deserialize)]
pub struct DeleteVectorsRequest {
    pub ids: Vec<String>,
}

#[derive(Serialize)]
pub struct MultiDeleteResponse {
    pub deleted_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f32>,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum DeleteResultsResponse {
    Single(DeleteResponse),
    Multi(MultiDeleteResponse),
}

#[derive(Serialize)]
pub struct CountResponse {
    pub count: usize,
}

// =============================================================================
// EMBEDDINGS
// =============================================================================

// Request to embed text and store as a vector
#[derive(Deserialize)]
pub struct EmbedRequest {
    #[serde(default)]
    pub text: Option<String>, // Text to embed; if not provided, embedding will be generated from vector (if possible) or an error will be returned
    #[serde(default)]
    pub texts: Option<Vec<String>>, // List of texts to embed for batch embedding; if not provided, embeddings will be generated from vectors (if possible) or an error will be returned
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>, // Metadata to associate with the vector(s)
    #[serde(default)]
    pub metadata_list: Vec<HashMap<String, serde_json::Value>>, // For batch embedding, a list of metadata maps corresponding to each text
}

// Response from embedding and storing
#[derive(Serialize)]
pub struct EmbedResponse {
    pub id: String,
    pub embedding: Vec<f32>,
    pub tokens: Option<u32>,
}

#[derive(Serialize)]
pub struct MultiEmbedResponse {
    pub ids: Vec<String>,
    pub embeddings: Vec<Vec<f32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u32>,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum EmbedResultsResponse {
    Single(EmbedResponse),
    Multi(MultiEmbedResponse),
}


// Request to search by text query (auto-embeds)
#[derive(Deserialize)]
pub struct TextSearchRequest {
    pub query: String,
    #[serde(default = "default_k")]
    pub k: usize,
    #[serde(default)]
    pub metric: Option<String>,
    #[serde(default)]
    pub ef: Option<usize>,
    #[serde(default)]
    pub nprobe: Option<usize>,
    #[serde(default)]
    pub overfetch: Option<usize>,
    #[serde(default)]
    pub preset: Option<String>,
}

// =============================================================================
// UPSERT
// =============================================================================

#[derive(Deserialize)]
pub struct UpsertRequest {
    pub id: Option<String>,  // If provided, use this ID; otherwise generate new
    pub vector: Vec<f32>,
    pub text: String,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub normalize: bool,  // Whether to normalize the vector
}

#[derive(Serialize)]
pub struct UpsertResponse {
    pub id: String,
    pub created: bool,  // true if inserted, false if updated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f32>,
}

// =============================================================================
// METRICS
// =============================================================================
pub mod range;

#[derive(Serialize)]
pub struct MetricsResponse {
    pub total_collections: usize,
    pub total_vectors: usize,
    pub collections: Vec<CollectionMetrics>,
    pub app_config: crate::config::AppConfig,
    pub wal_stats: Vec<WalStats>,
    pub embedding: EmbeddingMetricsResponse,
}

#[derive(Serialize)]
pub struct CollectionMetrics {
    pub name: String,
    pub vector_count: usize, 
    pub index_type: String,
    pub memory_usage_bytes: usize, // Memory usage of the index for this collection
    pub insert_latency_ms: Option<f32>, // Average latency for insert operations in this collection
    pub search_latency_ms: Option<f32>, // Average latency for search operations in this collection
    pub lock_read_ms: Option<f32>, // Average latency for acquiring read locks in this collection
    pub lock_write_ms: Option<f32>, // Average latency for acquiring write locks in this collection
    pub search_overfetch: Option<usize>, // Average overfetch factor used in search operations for this collection
    pub hnsw_ef_search: Option<usize>, // Average ef_search parameter used in HNSW search operations for this collection
    pub ivf_nprobe: Option<usize>, // Average nprobe parameter used in IVF search operations for this collection
}

#[derive(Serialize)]
pub struct WalStats {
    pub collection: String,
    pub last_checkpoint: Option<u64>,
    pub checkpoint_age_secs: Option<u64>, // Age of the last checkpoint in seconds
    pub wal_size_bytes: Option<u64>, // Total size of the WAL file for this collection in bytes
}

#[derive(Serialize)]
pub struct EmbeddingMetricsResponse {
    pub requests: u64,
    pub texts: u64,
    pub total_tokens: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_latency_ms: Option<f32>,
}

// =============================================================================
// DUPLICATE DETECTION
// =============================================================================

#[derive(Deserialize)]
pub struct DuplicateRequest {
    #[serde(default)]
    pub metric: Option<String>,
    #[serde(default = "default_dup_threshold")]
    pub threshold: f32,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub k: Option<usize>,
    #[serde(default)]
    pub ef: Option<usize>,
    #[serde(default)]
    pub nprobe: Option<usize>,
}

fn default_dup_threshold() -> f32 { 0.95 }

#[derive(Serialize)]
pub struct DuplicatePair {
    pub id_a: String,
    pub id_b: String,
    pub score: f32,
}

#[derive(Serialize)]
pub struct DuplicateResponse {
    pub pairs: Vec<DuplicatePair>,
}

// =============================================================================
// INDEX STATISTICS
// =============================================================================

#[derive(Serialize)]
pub struct IndexStatsResponse {
    pub index_type: String,
    pub total_vectors: usize, // Total number of vectors indexed
    pub memory_usage_bytes: usize, // Approximate memory usage of the index in bytes
    pub details: serde_json::Value, // Index-specific details as a JSON value (e.g., HNSW layer sizes, IVF cluster counts)
}

#[derive(Serialize)]
pub struct RebuildIndexResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f32>,
}

#[derive(Serialize)]
pub struct RebuildIndexStatusResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elapsed_ms: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// =============================================================================
// CONFIG
// =============================================================================

#[derive(Serialize)]
pub struct ConfigStatusResponse {
    pub app_config: crate::config::AppConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reloaded_at: Option<u64>,
}

#[derive(Serialize)]
pub struct ConfigReloadResponse {
    pub success: bool, 
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reloaded_at: Option<u64>, // Timestamp of when the config was reloaded (in seconds since UNIX epoch)
    pub app_config: crate::config::AppConfig,
}

// =============================================================================
// HEALTH / READY
// =============================================================================

#[derive(Serialize)]
pub struct CollectionHealth {
    pub name: String,
    pub loaded: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_checkpoint: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkpoint_age_secs: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wal_size_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_version: Option<u32>,
    pub integrity_ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct ReadyzResponse {
    pub ok: bool,
    pub version: String,
    pub data_dir: String,
    pub total_collections: usize,
    pub loaded_collections: usize,
    pub total_vectors: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk_total_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk_available_bytes: Option<u64>,
    pub collections: Vec<CollectionHealth>,
}
