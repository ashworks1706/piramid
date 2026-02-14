use axum::{extract::{Path, State}, response::Json};
use std::sync::atomic::Ordering;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use crate::error::{Result, ServerError};
use crate::validation;
use crate::server::metrics::record_lock_read;
use super::super::{
    state::{SharedState, RebuildState, RebuildJobStatus},
    types::*,
};

// GET /api/collections - list all loaded collections
pub async fn list_collections(State(state): State<SharedState>) -> Result<Json<CollectionsResponse>> {
    if state.shutting_down.load(Ordering::Relaxed) {
        return Err(ServerError::ServiceUnavailable("Server is shutting down".to_string()).into());
    }

    let mut infos = Vec::new();
    for entry in state.collections.iter() {
        let lock_start = std::time::Instant::now();
        let storage = entry.value().read();
        record_lock_read(state.latency_tracker.get(entry.key()).as_deref(), lock_start);
        let meta = storage.metadata();
        infos.push(CollectionInfo {
            name: entry.key().clone(),
            count: storage.count(),
            created_at: Some(meta.created_at),
            updated_at: Some(meta.updated_at),
            dimensions: meta.dimensions,
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

    // Validate collection name
    validation::validate_collection_name(&req.name)?;

    state.get_or_create_collection(&req.name)?;
    
    let storage_ref = state.collections.get(&req.name)
        .ok_or_else(|| ServerError::Internal("Collection not found after creation".into()))?;
    let lock_start = std::time::Instant::now();
    let storage = storage_ref.read();
    record_lock_read(state.latency_tracker.get(&req.name).as_deref(), lock_start);
    let meta = storage.metadata();
    
    Ok(Json(CollectionInfo { 
        name: req.name,
        count: storage.count(),
        created_at: Some(meta.created_at),
        updated_at: Some(meta.updated_at),
        dimensions: meta.dimensions,
    }))
}

// GET /api/collections/:name - get info about one collection
pub async fn get_collection(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
) -> Result<Json<CollectionInfo>> {
    if state.shutting_down.load(Ordering::Relaxed) {
        return Err(ServerError::ServiceUnavailable("Server is shutting down".to_string()).into());
    }

    state.get_or_create_collection(&collection)?;
    
    let storage_ref = state.collections.get(&collection)
        .ok_or_else(|| ServerError::NotFound("Collection not found".into()))?;
    let lock_start = std::time::Instant::now();
    let storage = storage_ref.read();
    record_lock_read(state.latency_tracker.get(&collection).as_deref(), lock_start);
    let meta = storage.metadata();
    
    Ok(Json(CollectionInfo { 
        name: collection,
        count: storage.count(),
        created_at: Some(meta.created_at),
        updated_at: Some(meta.updated_at),
        dimensions: meta.dimensions,
    }))
}

// DELETE /api/collections/:name - remove a collection
pub async fn delete_collection(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
) -> Result<Json<DeleteResponse>> {
    if state.shutting_down.load(Ordering::Relaxed) {
        return Err(ServerError::ServiceUnavailable("Server is shutting down".to_string()).into());
    }

    let existed = state.collections.remove(&collection).is_some();
    
    if existed {
        let path = format!("{}/{}.db", state.data_dir, collection);
        std::fs::remove_file(&path).ok();
    }
    
    Ok(Json(DeleteResponse { 
        deleted: existed,
        latency_ms: None,  // Collection deletion is a filesystem operation
    }))
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
    let lock_start = std::time::Instant::now();
    let storage = storage_ref.read();
    record_lock_read(state.latency_tracker.get(&collection).as_deref(), lock_start);
    let count = storage.count();
    
    Ok(Json(CountResponse { count }))
}

// GET /api/collections/:name/index/stats - get index statistics
pub async fn index_stats(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
) -> Result<Json<IndexStatsResponse>> {
    if state.shutting_down.load(Ordering::Relaxed) {
        return Err(ServerError::ServiceUnavailable("Server is shutting down".to_string()).into());
    }

    state.get_or_create_collection(&collection)?;
    
    let storage_ref = state.collections.get(&collection)
        .ok_or_else(|| ServerError::NotFound("Collection not found".into()))?;
    let lock_start = std::time::Instant::now();
    let storage = storage_ref.read();
    record_lock_read(state.latency_tracker.get(&collection).as_deref(), lock_start);
    
    let stats = storage.vector_index().stats();
    
    Ok(Json(IndexStatsResponse {
        index_type: stats.index_type.to_string(),
        total_vectors: stats.total_vectors,
        memory_usage_bytes: stats.memory_usage_bytes,
        details: serde_json::to_value(&stats.details).unwrap_or(serde_json::json!({})),
    }))
}


// POST /api/collections/:name/index/rebuild - trigger index rebuild
pub async fn rebuild_index(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
) -> Result<Json<RebuildIndexResponse>> {
    if state.shutting_down.load(Ordering::Relaxed) {
        return Err(ServerError::ServiceUnavailable("Server is shutting down".to_string()).into());
    }

    state.get_or_create_collection(&collection)?;
    
    let storage_ref = state.collections.get(&collection)
        .ok_or_else(|| ServerError::NotFound("Collection not found".into()))?;

    let started_at = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    state.rebuild_jobs.insert(collection.clone(), RebuildJobStatus {
        status: RebuildState::Running,
        started_at,
        finished_at: None,
        error: None,
        elapsed_ms: None,
    });

    // Run rebuild in background to avoid blocking the request.
    let collection_name = collection.clone();
    let storage_ref_clone = storage_ref.clone();
    let jobs = state.rebuild_jobs.clone();

    tokio::task::spawn_blocking(move || {
        let mut storage = storage_ref_clone.write();
        let start = Instant::now();
        if let Err(e) = storage.rebuild_index() {
            tracing::error!(collection=%collection_name, error=%e, "index_rebuild_failed");
            let finished = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            jobs.insert(collection_name.clone(), RebuildJobStatus {
                status: RebuildState::Failed,
                started_at,
                finished_at: Some(finished),
                error: Some(e.to_string()),
                elapsed_ms: Some(start.elapsed().as_millis()),
            });
        } else {
            tracing::info!(
                collection=%collection_name,
                elapsed_ms = start.elapsed().as_millis(),
                "index_rebuild_complete"
            );
            let finished = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            jobs.insert(collection_name.clone(), RebuildJobStatus {
                status: RebuildState::Completed,
                started_at,
                finished_at: Some(finished),
                error: None,
                elapsed_ms: Some(start.elapsed().as_millis()),
            });
        }
    });

    Ok(Json(RebuildIndexResponse { 
        success: true,
        latency_ms: None,
    }))
}

// GET /api/collections/:name/index/rebuild/status - check rebuild status
pub async fn rebuild_index_status(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
) -> Result<Json<RebuildIndexStatusResponse>> {
    if state.shutting_down.load(Ordering::Relaxed) {
        return Err(ServerError::ServiceUnavailable("Server is shutting down".to_string()).into());
    }

    if let Some(job) = state.rebuild_jobs.get(&collection) {
        let status_str = match job.status {
            RebuildState::Running => "running",
            RebuildState::Completed => "completed",
            RebuildState::Failed => "failed",
        };
        Ok(Json(RebuildIndexStatusResponse {
            status: status_str.to_string(),
            started_at: Some(job.started_at),
            finished_at: job.finished_at,
            elapsed_ms: job.elapsed_ms.map(|ms| ms as f32),
            error: job.error.clone(),
        }))
    } else {
        Err(ServerError::NotFound("No rebuild job found for this collection".into()).into())
    }
}
