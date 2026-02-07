use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use std::collections::HashMap;
use uuid::Uuid;

use crate::{Metric, MetadataValue, VectorEntry};
use super::state::SharedState;
use super::types::*;
use super::lock_helpers::LockHelper;

// Convert JSON values to internal Metadata type
fn json_to_metadata(json: HashMap<String, serde_json::Value>) -> crate::Metadata {
    let mut metadata = crate::Metadata::new();
    
    for (k, v) in json {
        let value = match v {
            serde_json::Value::String(s) => MetadataValue::String(s),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    MetadataValue::Integer(i)
                } else if let Some(f) = n.as_f64() {
                    MetadataValue::Float(f)
                } else {
                    continue;
                }
            }
            serde_json::Value::Bool(b) => MetadataValue::Boolean(b),
            serde_json::Value::Null => MetadataValue::Null,
            _ => continue,
        };
        metadata.insert(k, value);
    }
    
    metadata
}

// Convert internal Metadata to JSON for responses
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

// Parse similarity metric from string
fn parse_metric(s: Option<String>) -> Metric {
    match s.as_deref() {  // Option<String> → Option<&str>
        Some("euclidean") => Metric::Euclidean,
        Some("dot") | Some("dot_product") => Metric::DotProduct,
        _ => Metric::Cosine,  // default
    }
}

// GET /api/health - simple liveness check
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),  // reads from Cargo.toml at compile time
    })
}

// GET /api/health/embeddings - check if embeddings are available
pub async fn health_embeddings(State(state): State<SharedState>) -> StatusCode {
    match state.embedder {
        Some(_) => StatusCode::OK,
        None => StatusCode::SERVICE_UNAVAILABLE,
    }
}


// GET /api/collections - list all loaded collections
pub async fn list_collections(State(state): State<SharedState>) -> Result<Json<CollectionsResponse>, (StatusCode, String)> {
    // read() = shared lock (many readers allowed)
    let collections = state.collections.read_or_err()?;
    
    let infos: Vec<CollectionInfo> = collections
        .iter()
        .map(|(name, storage)| CollectionInfo {
            name: name.clone(),
            count: storage.count(),
        })
        .collect();
    
    Ok(Json(CollectionsResponse { collections: infos }))
}

// POST /api/collections - create a new collection
pub async fn create_collection(
    State(state): State<SharedState>,
    Json(req): Json<CreateCollectionRequest>,
) -> Result<Json<CollectionInfo>, (StatusCode, String)> {
    state.get_or_create_collection(&req.name)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    
    let collections = state.collections.read_or_err()?;
    let count = collections.get(&req.name).map(|s| s.count()).unwrap_or(0);
    
    Ok(Json(CollectionInfo { name: req.name, count }))
}

// GET /api/collections/:name - get info about one collection
pub async fn get_collection(
    State(state): State<SharedState>,
    Path(name): Path<String>,
) -> Result<Json<CollectionInfo>, (StatusCode, String)> {
    state.get_or_create_collection(&name)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    
    let collections = state.collections.read_or_err()?;
    let count = collections.get(&name).map(|s| s.count()).unwrap_or(0);
    
    Ok(Json(CollectionInfo { name, count }))
}

// DELETE /api/collections/:name - remove a collection
pub async fn delete_collection(
    State(state): State<SharedState>,
    Path(name): Path<String>,
) -> Result<Json<DeleteResponse>, (StatusCode, String)> {
    let mut collections = state.collections.write_or_err()?;
    let existed = collections.remove(&name).is_some();
    
    // also delete the file
    if existed {
        let path = format!("{}/{}.db", state.data_dir, name);
        std::fs::remove_file(&path).ok();
    }
    
    Ok(Json(DeleteResponse { deleted: existed }))
}

// GET /api/collections/:name/count - just the count
pub async fn collection_count(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
) -> Result<Json<CountResponse>, (StatusCode, String)> {
    state.get_or_create_collection(&collection)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    
    let collections = state.collections.read_or_err()?;
    let count = collections.get(&collection).map(|s| s.count()).unwrap_or(0);
    
    Ok(Json(CountResponse { count }))
}

// POST /api/collections/:collection/vectors - store a new vector
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
    let mut collections = state.collections.write_or_err()?;
    let storage = collections.get_mut(&collection)
        .ok_or((StatusCode::NOT_FOUND, "Collection not found".to_string()))?;
    
    let id = storage.store(entry)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    Ok(Json(StoreVectorResponse { id: id.to_string() }))
}

// POST /api/collections/:collection/vectors/batch - store multiple vectors at once
pub async fn store_vectors_batch(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Json(req): Json<BatchStoreVectorRequest>,
) -> Result<Json<BatchStoreVectorResponse>, (StatusCode, String)> {
    if req.vectors.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "No vectors provided".to_string()));
    }

    if req.texts.len() != req.vectors.len() {
        return Err((StatusCode::BAD_REQUEST, "vectors and texts length mismatch".to_string()));
    }

    state.get_or_create_collection(&collection)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    // Build entries
    let mut entries = Vec::with_capacity(req.vectors.len());
    for (idx, vector) in req.vectors.into_iter().enumerate() {
        let metadata = if idx < req.metadata.len() {
            json_to_metadata(req.metadata[idx].clone())
        } else {
            crate::Metadata::new()
        };
        
        let entry = VectorEntry::with_metadata(
            vector,
            req.texts[idx].clone(),
            metadata,
        );
        entries.push(entry);
    }

    // Store in batch
    let mut collections = state.collections.write_or_err()?;
    let storage = collections.get_mut(&collection)
        .ok_or((StatusCode::NOT_FOUND, "Collection not found".to_string()))?;

    let ids = storage.store_batch(entries)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let count = ids.len();
    let ids_str: Vec<String> = ids.into_iter().map(|id| id.to_string()).collect();

    Ok(Json(BatchStoreVectorResponse { ids: ids_str, count }))
}

// GET /api/collections/:collection/vectors/:id - get one vector
pub async fn get_vector(
    State(state): State<SharedState>,
    Path((collection, id)): Path<(String, String)>,  // tuple extraction!
) -> Result<Json<VectorResponse>, (StatusCode, String)> {
    state.get_or_create_collection(&collection)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid UUID".to_string()))?;
    
    let collections = state.collections.read_or_err()?;
    let storage = collections.get(&collection)
        .ok_or((StatusCode::NOT_FOUND, "Collection not found".to_string()))?;
    
    let entry = storage.get(&uuid)
        .ok_or((StatusCode::NOT_FOUND, "Vector not found".to_string()))?;
    
    Ok(Json(VectorResponse {
        id: entry.id.to_string(),
        vector: entry.get_vector(),  // Dequantize
        text: entry.text,
        metadata: metadata_to_json(&entry.metadata),
    }))
}

// GET /api/collections/:collection/vectors?limit=100&offset=0 - list vectors
pub async fn list_vectors(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Query(params): Query<ListVectorsQuery>,
) -> Result<Json<Vec<VectorResponse>>, (StatusCode, String)> {
    state.get_or_create_collection(&collection)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    
    let collections = state.collections.read_or_err()?;
    let storage = collections.get(&collection)
        .ok_or((StatusCode::NOT_FOUND, "Collection not found".to_string()))?;
    
    // skip() and take() for pagination
    let vectors: Vec<VectorResponse> = storage.get_all()
        .into_iter()
        .skip(params.offset)
        .take(params.limit)
        .map(|e| VectorResponse {
            id: e.id.to_string(),
            vector: e.get_vector(),  // Dequantize
            text: e.text.clone(),
            metadata: metadata_to_json(&e.metadata),
        })
        .collect();
    
    Ok(Json(vectors))
}

// DELETE /api/collections/:collection/vectors/:id - delete a vector
pub async fn delete_vector(
    State(state): State<SharedState>,
    Path((collection, id)): Path<(String, String)>,
) -> Result<Json<DeleteResponse>, (StatusCode, String)> {
    state.get_or_create_collection(&collection)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid UUID".to_string()))?;
    
    let mut collections = state.collections.write_or_err()?;
    let storage = collections.get_mut(&collection)
        .ok_or((StatusCode::NOT_FOUND, "Collection not found".to_string()))?;
    
    let deleted = storage.delete(&uuid)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    Ok(Json(DeleteResponse { deleted }))
}

// POST /api/collections/:collection/search - similarity search
pub async fn search_vectors(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Json(req): Json<SearchRequest>,
) -> Result<Json<SearchResponse>, (StatusCode, String)> {
    state.get_or_create_collection(&collection)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    
    let metric = parse_metric(req.metric);
    
    let collections = state.collections.read_or_err()?;
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

// =============================================================================
// EMBEDDING ENDPOINTS
// =============================================================================

// POST /api/collections/:collection/embed - embed text and store
pub async fn embed_text(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Json(req): Json<EmbedRequest>,
) -> Result<Json<EmbedResponse>, (StatusCode, String)> {
    state.get_or_create_collection(&collection)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    // Get embedder from state
    let embedder = state.embedder.as_ref()
        .ok_or((StatusCode::SERVICE_UNAVAILABLE, "Embedding service not configured".to_string()))?;

    // Generate embedding
    let response = embedder.embed(&req.text).await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Embedding failed: {}", e)))?;

    // Store the vector
    let metadata = json_to_metadata(req.metadata);
    let entry = VectorEntry::with_metadata(
        response.embedding.clone(),
        req.text,
        metadata,
    );

    let mut collections = state.collections.write_or_err()?;
    let storage = collections.get_mut(&collection)
        .ok_or((StatusCode::NOT_FOUND, "Collection not found".to_string()))?;

    let id = storage.store(entry)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(EmbedResponse {
        id: id.to_string(),
        embedding: response.embedding,
        tokens: response.tokens,
    }))
}

// POST /api/collections/:collection/embed/batch - batch embed texts
pub async fn embed_batch(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Json(req): Json<EmbedBatchRequest>,
) -> Result<Json<EmbedBatchResponse>, (StatusCode, String)> {
    if req.texts.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "No texts provided".to_string()));
    }

    state.get_or_create_collection(&collection)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    // Get embedder from state
    let embedder = state.embedder.as_ref()
        .ok_or((StatusCode::SERVICE_UNAVAILABLE, "Embedding service not configured".to_string()))?;

    // Generate embeddings in batch
    let responses = embedder.embed_batch(&req.texts).await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Batch embedding failed: {}", e)))?;

    // Store all vectors
    let mut collections = state.collections.write_or_err()?;
    let storage = collections.get_mut(&collection)
        .ok_or((StatusCode::NOT_FOUND, "Collection not found".to_string()))?;

    let mut ids = Vec::with_capacity(responses.len());
    let mut total_tokens = 0u32;

    for (idx, response) in responses.into_iter().enumerate() {
        let metadata = if idx < req.metadata.len() {
            json_to_metadata(req.metadata[idx].clone())
        } else {
            crate::Metadata::new()
        };

        let entry = VectorEntry::with_metadata(
            response.embedding,
            req.texts[idx].clone(),
            metadata,
        );

        let id = storage.store(entry)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        ids.push(id.to_string());
        if let Some(tokens) = response.tokens {
            total_tokens += tokens;
        }
    }

    Ok(Json(EmbedBatchResponse {
        ids,
        total_tokens: if total_tokens > 0 { Some(total_tokens) } else { None },
    }))
}

// POST /api/collections/:collection/search/text - search by text query
pub async fn search_by_text(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Json(req): Json<TextSearchRequest>,
) -> Result<Json<SearchResponse>, (StatusCode, String)> {
    state.get_or_create_collection(&collection)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    // Get embedder from state
    let embedder = state.embedder.as_ref()
        .ok_or((StatusCode::SERVICE_UNAVAILABLE, "Embedding service not configured".to_string()))?;

    // Generate embedding for query
    let response = embedder.embed(&req.query).await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Embedding failed: {}", e)))?;

    // Search with the embedding
    let metric = parse_metric(req.metric);

    let collections = state.collections.read_or_err()?;
    let storage = collections.get(&collection)
        .ok_or((StatusCode::NOT_FOUND, "Collection not found".to_string()))?;

    let results: Vec<SearchResultResponse> = storage
        .search(&response.embedding, req.k, metric)
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
