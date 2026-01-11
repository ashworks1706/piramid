//! These structs define the JSON shape for API communication.
//! Serde does the heavy lifting: Serialize = Rust → JSON, Deserialize = JSON → Rust.

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

/// What the client sends to store a vector
#[derive(Deserialize)]
pub struct StoreVectorRequest {
    pub vector: Vec<f32>,
    pub text: String,
    #[serde(default)]  // if missing in JSON, use Default (empty HashMap)
    pub metadata: HashMap<String, serde_json::Value>,
}

/// What we return after storing
#[derive(Serialize)]
pub struct StoreVectorResponse {
    pub id: String,
}

/// Full vector data returned to client
#[derive(Serialize)]
pub struct VectorResponse {
    pub id: String,
    pub vector: Vec<f32>,
    pub text: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Query params for listing vectors: ?limit=100&offset=0
#[derive(Deserialize)]
pub struct ListVectorsQuery {
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

fn default_limit() -> usize { 100 }

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
pub struct SearchResultResponse {
    pub id: String,
    pub score: f32,
    pub text: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResultResponse>,
}

// =============================================================================
// COMMON
// =============================================================================

#[derive(Serialize)]
pub struct DeleteResponse {
    pub deleted: bool,
}

#[derive(Serialize)]
pub struct CountResponse {
    pub count: usize,
}

// =============================================================================
// EMBEDDINGS
// =============================================================================

/// Request to embed text and store as a vector
#[derive(Deserialize)]
pub struct EmbedRequest {
    pub text: String,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Response from embedding and storing
#[derive(Serialize)]
pub struct EmbedResponse {
    pub id: String,
    pub embedding: Vec<f32>,
    pub tokens: Option<u32>,
}

/// Request for batch embedding
#[derive(Deserialize)]
pub struct EmbedBatchRequest {
    pub texts: Vec<String>,
    #[serde(default)]
    pub metadata: Vec<HashMap<String, serde_json::Value>>,
}

/// Response from batch embedding
#[derive(Serialize)]
pub struct EmbedBatchResponse {
    pub ids: Vec<String>,
    pub total_tokens: Option<u32>,
}

/// Request to search by text query (auto-embeds)
#[derive(Deserialize)]
pub struct TextSearchRequest {
    pub query: String,
    #[serde(default = "default_k")]
    pub k: usize,
    #[serde(default)]
    pub metric: Option<String>,
}
