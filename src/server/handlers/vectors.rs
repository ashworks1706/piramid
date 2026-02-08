use axum::{extract::{Path, Query, State}, response::Json};
use uuid::Uuid;
use std::sync::atomic::Ordering;
use std::time::Duration;
use crate::{Metric, Document};
use crate::error::{Result, ServerError};
use super::super::{
    state::SharedState,
    types::*,
    sync::LockHelper,
    helpers::{json_to_metadata, metadata_to_json},
};

const LOCK_TIMEOUT: Duration = Duration::from_secs(5);

// Parse similarity metric from string
fn parse_metric(s: Option<String>) -> Metric {
    match s.as_deref() {
        Some("euclidean") => Metric::Euclidean,
        Some("dot") | Some("dot_product") => Metric::DotProduct,
        _ => Metric::Cosine,
    }
}

// POST /api/collections/:collection/vectors - store a new vector
pub async fn insert_vector(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Json(req): Json<InsertRequest>,
) -> Result<Json<InsertResponse>> {
    if state.shutting_down.load(Ordering::Relaxed) {
        return Err(ServerError::ServiceUnavailable("Server is shutting down".to_string()).into());
    }

    state.get_or_create_collection(&collection)?;
    
    let metadata = json_to_metadata(req.metadata);
    let entry = Document::with_metadata(req.vector, req.text, metadata);
    
    let storage_ref = state.collections.get(&collection)
        .ok_or_else(|| ServerError::NotFound(super::super::helpers::COLLECTION_NOT_FOUND.to_string()))?;
    let mut storage = storage_ref.write_with_timeout(LOCK_TIMEOUT)?;
    
    let id = storage.insert(entry)?;
    
    Ok(Json(InsertResponse { id: id.to_string() }))
}

// POST /api/collections/:collection/vectors/batch - store multiple vectors at once
pub async fn insert_vectors_batch(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Json(req): Json<BatchInsertRequest>,
) -> Result<Json<BatchInsertResponse>> {
    if state.shutting_down.load(Ordering::Relaxed) {
        return Err(ServerError::ServiceUnavailable("Server is shutting down".to_string()).into());
    }

    if req.vectors.is_empty() {
        return Err(ServerError::InvalidRequest("No vectors provided".to_string()).into());
    }

    if req.texts.len() != req.vectors.len() {
        return Err(ServerError::InvalidRequest("vectors and texts length mismatch".to_string()).into());
    }

    state.get_or_create_collection(&collection)?;

    // Build entries
    let mut entries = Vec::with_capacity(req.vectors.len());
    for (idx, vector) in req.vectors.into_iter().enumerate() {
        let metadata = if idx < req.metadata.len() {
            json_to_metadata(req.metadata[idx].clone())
        } else {
            crate::Metadata::new()
        };
        
        let entry = Document::with_metadata(
            vector,
            req.texts[idx].clone(),
            metadata,
        );
        entries.push(entry);
    }

    // Store in batch
    let storage_ref = state.collections.get(&collection)
        .ok_or_else(|| ServerError::NotFound(super::super::helpers::COLLECTION_NOT_FOUND.to_string()))?;
    let mut storage = storage_ref.write_with_timeout(LOCK_TIMEOUT)?;

    let ids: Vec<Uuid> = storage.insert_batch(entries)?;

    let count = ids.len();
    let ids_str: Vec<String> = ids.into_iter().map(|id: Uuid| id.to_string()).collect();

    Ok(Json(BatchInsertResponse { ids: ids_str, count }))
}

// GET /api/collections/:collection/vectors/:id - get one vector
pub async fn get_vector(
    State(state): State<SharedState>,
    Path((collection, id)): Path<(String, String)>,
) -> Result<Json<VectorResponse>> {
    if state.shutting_down.load(Ordering::Relaxed) {
        return Err(ServerError::ServiceUnavailable("Server is shutting down".to_string()).into());
    }

    state.get_or_create_collection(&collection)?;
    
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| ServerError::InvalidRequest("Invalid UUID".to_string()))?;
    
    let storage_ref = state.collections.get(&collection)
        .ok_or_else(|| ServerError::NotFound(super::super::helpers::COLLECTION_NOT_FOUND.to_string()))?;
    let storage = storage_ref.read_with_timeout(LOCK_TIMEOUT)?;
    
    let entry = storage.get(&uuid)
        .ok_or(ServerError::NotFound(super::super::helpers::VECTOR_NOT_FOUND.to_string()))?;
    
    Ok(Json(VectorResponse {
        id: entry.id.to_string(),
        vector: entry.get_vector(),
        text: entry.text,
        metadata: metadata_to_json(&entry.metadata),
    }))
}

// GET /api/collections/:collection/vectors?limit=100&offset=0 - list vectors
pub async fn list_vectors(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Query(params): Query<ListVectorsQuery>,
) -> Result<Json<Vec<VectorResponse>>> {
    if state.shutting_down.load(Ordering::Relaxed) {
        return Err(ServerError::ServiceUnavailable("Server is shutting down".to_string()).into());
    }

    state.get_or_create_collection(&collection)?;
    
    let storage_ref = state.collections.get(&collection)
        .ok_or_else(|| ServerError::NotFound(super::super::helpers::COLLECTION_NOT_FOUND.to_string()))?;
    let storage = storage_ref.read_with_timeout(LOCK_TIMEOUT)?;
    
    let vectors: Vec<VectorResponse> = storage.get_all()
        .into_iter()
        .skip(params.offset)
        .take(params.limit)
        .map(|e| VectorResponse {
            id: e.id.to_string(),
            vector: e.get_vector(),
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
) -> Result<Json<DeleteResponse>> {
    if state.shutting_down.load(Ordering::Relaxed) {
        return Err(ServerError::ServiceUnavailable("Server is shutting down".to_string()).into());
    }

    state.get_or_create_collection(&collection)?;
    
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| ServerError::InvalidRequest("Invalid UUID".to_string()))?;
    
    let storage_ref = state.collections.get(&collection)
        .ok_or_else(|| ServerError::NotFound(super::super::helpers::COLLECTION_NOT_FOUND.to_string()))?;
    let mut storage = storage_ref.write_with_timeout(LOCK_TIMEOUT)?;
    
    let deleted = storage.delete(&uuid)?;
    
    Ok(Json(DeleteResponse { deleted }))
}

// POST /api/collections/:collection/search - search for similar vectors
pub async fn search_vectors(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Json(req): Json<SearchRequest>,
) -> Result<Json<SearchResponse>> {
    if state.shutting_down.load(Ordering::Relaxed) {
        return Err(ServerError::ServiceUnavailable("Server is shutting down".to_string()).into());
    }

    state.get_or_create_collection(&collection)?;
    
    let storage_ref = state.collections.get(&collection)
        .ok_or_else(|| ServerError::NotFound(super::super::helpers::COLLECTION_NOT_FOUND.to_string()))?;
    let storage = storage_ref.read_with_timeout(LOCK_TIMEOUT)?;
    
    let metric = parse_metric(req.metric);
    let results = storage.search(&req.vector, req.k, metric);
    
    let search_results: Vec<HitResponse> = results
        .into_iter()
        .map(|r| HitResponse {
            id: r.id.to_string(),
            score: r.score,
            text: r.text,
            metadata: metadata_to_json(&r.metadata),
        })
        .collect();
    
    Ok(Json(SearchResponse { results: search_results }))
}
