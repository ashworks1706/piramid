use axum::{extract::{Path, State}, response::Json};
use std::sync::atomic::Ordering;
use std::time::Duration;
use crate::error::{Result, ServerError};
use super::super::{
    state::SharedState,
    types::*,
    sync::LockHelper,
};

const LOCK_TIMEOUT: Duration = Duration::from_secs(5);

// GET /api/collections - list all loaded collections
pub async fn list_collections(State(state): State<SharedState>) -> Result<Json<CollectionsResponse>> {
    if state.shutting_down.load(Ordering::Relaxed) {
        return Err(ServerError::ServiceUnavailable("Server is shutting down".to_string()).into());
    }

    let mut infos = Vec::new();
    for entry in state.collections.iter() {
        let storage = entry.value().read_with_timeout(LOCK_TIMEOUT)?;
        infos.push(CollectionInfo {
            name: entry.key().clone(),
            count: storage.count(),
        });
    }
    
    Ok(Json(CollectionsResponse { collections: infos }))
}

// POST /api/collections - create a new collection
pub async fn create_collection(
    State(state): State<SharedState>,
    Json(req): Json<CreateCollectionRequest>,
) -> Result<Json<CollectionInfo>> {
    if state.shutting_down.load(Ordering::Relaxed) {
        return Err(ServerError::ServiceUnavailable("Server is shutting down".to_string()).into());
    }

    state.get_or_create_collection(&req.name)?;
    
    let storage_ref = state.collections.get(&req.name)
        .ok_or_else(|| ServerError::Internal("Collection not found after creation".into()))?;
    let storage = storage_ref.read_with_timeout(LOCK_TIMEOUT)?;
    let count = storage.count();
    
    Ok(Json(CollectionInfo { name: req.name, count }))
}

// GET /api/collections/:name - get info about one collection
pub async fn get_collection(
    State(state): State<SharedState>,
    Path(name): Path<String>,
) -> Result<Json<CollectionInfo>> {
    if state.shutting_down.load(Ordering::Relaxed) {
        return Err(ServerError::ServiceUnavailable("Server is shutting down".to_string()).into());
    }

    state.get_or_create_collection(&name)?;
    
    let storage_ref = state.collections.get(&name)
        .ok_or_else(|| ServerError::NotFound("Collection not found".into()))?;
    let storage = storage_ref.read_with_timeout(LOCK_TIMEOUT)?;
    let count = storage.count();
    
    Ok(Json(CollectionInfo { name, count }))
}

// DELETE /api/collections/:name - remove a collection
pub async fn delete_collection(
    State(state): State<SharedState>,
    Path(name): Path<String>,
) -> Result<Json<DeleteResponse>> {
    if state.shutting_down.load(Ordering::Relaxed) {
        return Err(ServerError::ServiceUnavailable("Server is shutting down".to_string()).into());
    }

    let existed = state.collections.remove(&name).is_some();
    
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
) -> Result<Json<CountResponse>> {
    if state.shutting_down.load(Ordering::Relaxed) {
        return Err(ServerError::ServiceUnavailable("Server is shutting down".to_string()).into());
    }

    state.get_or_create_collection(&collection)?;
    
    let storage_ref = state.collections.get(&collection)
        .ok_or_else(|| ServerError::NotFound("Collection not found".into()))?;
    let storage = storage_ref.read_with_timeout(LOCK_TIMEOUT)?;
    let count = storage.count();
    
    Ok(Json(CountResponse { count }))
}
