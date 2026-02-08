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
    pub version: &'static str,
}

// =============================================================================
// COLLECTIONS
// =============================================================================

#[derive(Serialize)]
pub struct CollectionInfo {
    pub name: String,
    pub count: usize,
    pub created_at: Option<u64>,
    pub updated_at: Option<u64>,
    pub dimensions: Option<usize>,
}

#[derive(Serialize)]
pub struct CollectionsResponse {
    pub collections: Vec<CollectionInfo>,
}

#[derive(Deserialize)]
pub struct CreateCollectionRequest {
    pub name: String,
}

// =============================================================================
// VECTORS
// =============================================================================

// What the client sends to store a vector
#[derive(Deserialize)]
pub struct InsertRequest {
    pub vector: Vec<f32>,
    pub text: String,
    #[serde(default)]  // if missing in JSON, use Default (empty HashMap)
    pub metadata: HashMap<String, serde_json::Value>,
    #[serde(default)]  // if missing, defaults to false
    pub normalize: bool,  // Whether to normalize the vector to unit length
}

// What we return after storing
#[derive(Serialize)]
pub struct InsertResponse {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f32>,
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
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

fn default_limit() -> usize { 100 }

// =============================================================================
// BATCH INSERT
// =============================================================================

// Batch store request
#[derive(Deserialize)]
pub struct BatchInsertRequest {
    pub vectors: Vec<Vec<f32>>,
    pub texts: Vec<String>,
    #[serde(default)]
    pub metadata: Vec<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    pub normalize: bool,  // Whether to normalize all vectors
}

// Batch store response
#[derive(Serialize)]
pub struct BatchInsertResponse {
    pub ids: Vec<String>,
    pub count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f32>,
}


// =============================================================================
// SEARCH
// =============================================================================

#[derive(Deserialize)]
pub struct SearchRequest {
    pub vector: Vec<f32>,
    #[serde(default = "default_k")]
    pub k: usize,  // how many results to return
    #[serde(default)]
    pub metric: Option<String>,  // "cosine", "euclidean", "dot"
}

fn default_k() -> usize { 10 }

#[derive(Serialize)]
pub struct HitResponse {
    pub id: String,
    pub score: f32,
    pub text: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Serialize)]
pub struct SearchResponse {
    pub results: Vec<HitResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f32>,
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

#[derive(Serialize)]
pub struct CountResponse {
    pub count: usize,
}

// =============================================================================
// BATCH DELETE
// =============================================================================

#[derive(Deserialize)]
pub struct BatchDeleteRequest {
    pub ids: Vec<String>,  // Vector IDs to delete
}

#[derive(Serialize)]
pub struct BatchDeleteResponse {
    pub deleted_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f32>,
}

// =============================================================================
// EMBEDDINGS
// =============================================================================

// Request to embed text and store as a vector
#[derive(Deserialize)]
pub struct EmbedRequest {
    pub text: String,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

// Response from embedding and storing
#[derive(Serialize)]
pub struct EmbedResponse {
    pub id: String,
    pub embedding: Vec<f32>,
    pub tokens: Option<u32>,
}

// Request for batch embedding
#[derive(Deserialize)]
pub struct EmbedBatchRequest {
    pub texts: Vec<String>,
    #[serde(default)]
    pub metadata: Vec<HashMap<String, serde_json::Value>>,
}

// Response from batch embedding
#[derive(Serialize)]
pub struct EmbedBatchResponse {
    pub ids: Vec<String>,
    pub total_tokens: Option<u32>,
}

// Request to search by text query (auto-embeds)
#[derive(Deserialize)]
pub struct TextSearchRequest {
    pub query: String,
    #[serde(default = "default_k")]
    pub k: usize,
    #[serde(default)]
    pub metric: Option<String>,
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
// BATCH SEARCH
// =============================================================================

#[derive(Deserialize)]
pub struct BatchSearchRequest {
    pub vectors: Vec<Vec<f32>>,
    #[serde(default = "default_k")]
    pub k: usize,
    #[serde(default)]
    pub metric: Option<String>,
}

#[derive(Serialize)]
pub struct BatchSearchResponse {
    pub results: Vec<Vec<HitResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f32>,
}

// =============================================================================
// METRICS
// =============================================================================

#[derive(Serialize)]
pub struct MetricsResponse {
    pub total_collections: usize,
    pub total_vectors: usize,
    pub collections: Vec<CollectionMetrics>,
}

#[derive(Serialize)]
pub struct CollectionMetrics {
    pub name: String,
    pub vector_count: usize,
    pub index_type: String,
    pub memory_usage_bytes: usize,
    pub insert_latency_ms: Option<f32>,
    pub search_latency_ms: Option<f32>,
}

// =============================================================================
// INDEX STATISTICS
// =============================================================================

#[derive(Serialize)]
pub struct IndexStatsResponse {
    pub index_type: String,
    pub total_vectors: usize,
    pub memory_usage_bytes: usize,
    pub details: serde_json::Value,
}
