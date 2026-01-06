//! HTTP request handlers
//!
//! Each function here handles one endpoint. The pattern:
//! 1. Extract what you need from the request (State, Path, Json, Query)
//! 2. Do the work (call into our library)
//! 3. Return JSON response or error
//!
//! Axum uses "extractors" - types that know how to pull data from requests.
//! Order matters! State first, then Path, then Query/Json.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use std::collections::HashMap;
use uuid::Uuid;

use crate::{DistanceMetric, MetadataValue, VectorEntry};
use super::state::SharedState;
use super::types::*;

// =============================================================================
// METADATA CONVERSION
// =============================================================================

/// Convert JSON values to our Metadata type
/// 
/// serde_json::Value is a tree of JSON. We flatten it to our enum.
/// This is the "adapter pattern" - translating between two representations.
fn json_to_metadata(json: HashMap<String, serde_json::Value>) -> crate::Metadata {
    let mut metadata = crate::Metadata::new();
    
    for (k, v) in json {
        let value = match v {
            serde_json::Value::String(s) => MetadataValue::String(s),
            serde_json::Value::Number(n) => {
                // JSON numbers could be int or float
                if let Some(i) = n.as_i64() {
                    MetadataValue::Integer(i)
                } else if let Some(f) = n.as_f64() {
                    MetadataValue::Float(f)
                } else {
                    continue;  // skip weird numbers
                }
            }
            serde_json::Value::Bool(b) => MetadataValue::Boolean(b),
            serde_json::Value::Null => MetadataValue::Null,
            _ => continue,  // skip arrays/objects for now
        };
        metadata.insert(k, value);
    }
    
    metadata
}

/// Convert our Metadata back to JSON for responses
fn metadata_to_json(metadata: &crate::Metadata) -> HashMap<String, serde_json::Value> {
    metadata
        .iter()
        .map(|(k, v)| {
            let json_val = match v {
                MetadataValue::String(s) => serde_json::Value::String(s.clone()),
                MetadataValue::Integer(i) => serde_json::json!(*i),
                MetadataValue::Float(f) => serde_json::json!(*f),
                MetadataValue::Boolean(b) => serde_json::Value::Bool(*b),
                MetadataValue::Null => serde_json::Value::Null,
                MetadataValue::Array(arr) => {
                    serde_json::Value::Array(arr.iter().map(|item| match item {
                        MetadataValue::String(s) => serde_json::Value::String(s.clone()),
                        MetadataValue::Integer(i) => serde_json::json!(*i),
                        MetadataValue::Float(f) => serde_json::json!(*f),
                        MetadataValue::Boolean(b) => serde_json::Value::Bool(*b),
                        _ => serde_json::Value::Null,
                    }).collect())
                }
            };
            (k.clone(), json_val)
        })
        .collect()
}

/// Parse distance metric from string
fn parse_metric(s: Option<String>) -> DistanceMetric {
    match s.as_deref() {  // Option<String> → Option<&str>
        Some("euclidean") => DistanceMetric::Euclidean,
        Some("dot") | Some("dot_product") => DistanceMetric::DotProduct,
        _ => DistanceMetric::Cosine,  // default
    }
}

// =============================================================================
// HEALTH
// =============================================================================

/// GET /api/health - simple liveness check
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),  // reads from Cargo.toml at compile time
    })
}

// =============================================================================
// COLLECTIONS
// =============================================================================

/// GET /api/collections - list all loaded collections
pub async fn list_collections(State(state): State<SharedState>) -> Json<CollectionsResponse> {
    // read() = shared lock (many readers allowed)
    let collections = state.collections.read().unwrap();
    
    let infos: Vec<CollectionInfo> = collections
        .iter()
        .map(|(name, storage)| CollectionInfo {
            name: name.clone(),
            count: storage.count(),
        })
        .collect();
    
    Json(CollectionsResponse { collections: infos })
}

/// POST /api/collections - create a new collection
pub async fn create_collection(
    State(state): State<SharedState>,
    Json(req): Json<CreateCollectionRequest>,
) -> Result<Json<CollectionInfo>, (StatusCode, String)> {
    state.get_or_create_collection(&req.name)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    
    let collections = state.collections.read().unwrap();
    let count = collections.get(&req.name).map(|s| s.count()).unwrap_or(0);
    
    Ok(Json(CollectionInfo { name: req.name, count }))
}

/// GET /api/collections/:name - get info about one collection
pub async fn get_collection(
    State(state): State<SharedState>,
    Path(name): Path<String>,
) -> Result<Json<CollectionInfo>, (StatusCode, String)> {
    state.get_or_create_collection(&name)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    
    let collections = state.collections.read().unwrap();
    let count = collections.get(&name).map(|s| s.count()).unwrap_or(0);
    
    Ok(Json(CollectionInfo { name, count }))
}

/// DELETE /api/collections/:name - remove a collection
pub async fn delete_collection(
    State(state): State<SharedState>,
    Path(name): Path<String>,
) -> Result<Json<DeleteResponse>, (StatusCode, String)> {
    let mut collections = state.collections.write().unwrap();
    let existed = collections.remove(&name).is_some();
    
    // also delete the file
    if existed {
        let path = format!("{}/{}.db", state.data_dir, name);
        std::fs::remove_file(&path).ok();
    }
    
    Ok(Json(DeleteResponse { deleted: existed }))
}

/// GET /api/collections/:name/count - just the count
pub async fn collection_count(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
) -> Result<Json<CountResponse>, (StatusCode, String)> {
    state.get_or_create_collection(&collection)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    
    let collections = state.collections.read().unwrap();
    let count = collections.get(&collection).map(|s| s.count()).unwrap_or(0);
    
    Ok(Json(CountResponse { count }))
}

// =============================================================================
// VECTORS
// =============================================================================

/// POST /api/collections/:collection/vectors - store a new vector
pub async fn store_vector(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Json(req): Json<StoreVectorRequest>,
) -> Result<Json<StoreVectorResponse>, (StatusCode, String)> {
    state.get_or_create_collection(&collection)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    
    let metadata = json_to_metadata(req.metadata);
    let entry = VectorEntry::with_metadata(req.vector, req.text, metadata);
    
    // write() = exclusive lock (we're modifying)
    let mut collections = state.collections.write().unwrap();
    let storage = collections.get_mut(&collection)
        .ok_or((StatusCode::NOT_FOUND, "Collection not found".to_string()))?;
    
    let id = storage.store(entry)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    Ok(Json(StoreVectorResponse { id: id.to_string() }))
}

/// GET /api/collections/:collection/vectors/:id - get one vector
pub async fn get_vector(
    State(state): State<SharedState>,
    Path((collection, id)): Path<(String, String)>,  // tuple extraction!
) -> Result<Json<VectorResponse>, (StatusCode, String)> {
    state.get_or_create_collection(&collection)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid UUID".to_string()))?;
    
    let collections = state.collections.read().unwrap();
    let storage = collections.get(&collection)
        .ok_or((StatusCode::NOT_FOUND, "Collection not found".to_string()))?;
    
    let entry = storage.get(&uuid)
        .ok_or((StatusCode::NOT_FOUND, "Vector not found".to_string()))?;
    
    Ok(Json(VectorResponse {
        id: entry.id.to_string(),
        vector: entry.vector,
        text: entry.text,
        metadata: metadata_to_json(&entry.metadata),
    }))
}

/// GET /api/collections/:collection/vectors?limit=100&offset=0 - list vectors
pub async fn list_vectors(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Query(params): Query<ListVectorsQuery>,
) -> Result<Json<Vec<VectorResponse>>, (StatusCode, String)> {
    state.get_or_create_collection(&collection)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    
    let collections = state.collections.read().unwrap();
    let storage = collections.get(&collection)
        .ok_or((StatusCode::NOT_FOUND, "Collection not found".to_string()))?;
    
    // skip() and take() for pagination
    let vectors: Vec<VectorResponse> = storage.get_all()
        .into_iter()
        .skip(params.offset)
        .take(params.limit)
        .map(|e| VectorResponse {
            id: e.id.to_string(),
            vector: e.vector.clone(),
            text: e.text.clone(),
            metadata: metadata_to_json(&e.metadata),
        })
        .collect();
    
    Ok(Json(vectors))
}

/// DELETE /api/collections/:collection/vectors/:id - delete a vector
pub async fn delete_vector(
    State(state): State<SharedState>,
    Path((collection, id)): Path<(String, String)>,
) -> Result<Json<DeleteResponse>, (StatusCode, String)> {
    state.get_or_create_collection(&collection)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid UUID".to_string()))?;
    
    let mut collections = state.collections.write().unwrap();
    let storage = collections.get_mut(&collection)
        .ok_or((StatusCode::NOT_FOUND, "Collection not found".to_string()))?;
    
    let deleted = storage.delete(&uuid)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    Ok(Json(DeleteResponse { deleted }))
}

// =============================================================================
// SEARCH
// =============================================================================

/// POST /api/collections/:collection/search - similarity search
pub async fn search_vectors(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Json(req): Json<SearchRequest>,
) -> Result<Json<SearchResponse>, (StatusCode, String)> {
    state.get_or_create_collection(&collection)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    
    let metric = parse_metric(req.metric);
    
    let collections = state.collections.read().unwrap();
    let storage = collections.get(&collection)
        .ok_or((StatusCode::NOT_FOUND, "Collection not found".to_string()))?;
    
    let results: Vec<SearchResultResponse> = storage
        .search(&req.vector, req.k, metric)
        .into_iter()
        .map(|r| SearchResultResponse {
            id: r.id.to_string(),
            score: r.score,
            text: r.text,
            metadata: metadata_to_json(&r.metadata),
        })
        .collect();
    
    Ok(Json(SearchResponse { results }))
}
