use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::error::{Result, ServerError};
use crate::metrics::record_lock_read;
use crate::metrics::Metric;
use crate::runtime::{RebuildJobStatus, RebuildState, SharedState};
use crate::server::types::*;
use crate::validation;

fn ensure_available(state: &SharedState) -> Result<()> {
    if state
        .shutting_down
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        return Err(ServerError::ServiceUnavailable("Server is shutting down".to_string()).into());
    }
    Ok(())
}

fn collection_info(name: String, storage: &crate::Collection) -> CollectionInfo {
    let meta = storage.metadata();
    CollectionInfo {
        name,
        count: storage.count(),
        created_at: Some(meta.created_at),
        updated_at: Some(meta.updated_at),
        dimensions: meta.dimensions,
    }
}

pub fn list_collections(state: &SharedState) -> Result<CollectionsResponse> {
    ensure_available(state)?;

    let mut collections = Vec::new();
    for (name, storage_ref) in state.registry.loaded_collections() {
        let lock_start = Instant::now();
        let storage = storage_ref.read();
        record_lock_read(state.registry.tracker(&name).as_deref(), lock_start);
        collections.push(collection_info(name, &storage));
    }

    Ok(CollectionsResponse { collections })
}

pub fn create_collection(
    state: &SharedState,
    req: CreateCollectionRequest,
) -> Result<CollectionInfo> {
    ensure_available(state)?;
    validation::validate_collection_name(&req.name)?;

    let storage_ref = state.get_or_create_collection(&req.name)?;
    let lock_start = Instant::now();
    let storage = storage_ref.read();
    record_lock_read(state.registry.tracker(&req.name).as_deref(), lock_start);
    Ok(collection_info(req.name, &storage))
}

pub fn get_collection(state: &SharedState, collection: String) -> Result<CollectionInfo> {
    ensure_available(state)?;

    let storage_ref = state.get_existing_collection(&collection)?;
    let lock_start = Instant::now();
    let storage = storage_ref.read();
    record_lock_read(state.registry.tracker(&collection).as_deref(), lock_start);
    Ok(collection_info(collection, &storage))
}

pub fn delete_collection(state: &SharedState, collection: String) -> Result<DeleteResponse> {
    ensure_available(state)?;

    let existed = state.registry.remove(&collection).is_some();
    if existed {
        let path = format!("{}/{}.db", state.data_dir, collection);
        std::fs::remove_file(&path).ok();
    }

    Ok(DeleteResponse {
        deleted: existed,
        latency_ms: None,
    })
}

pub fn collection_count(state: &SharedState, collection: String) -> Result<CountResponse> {
    ensure_available(state)?;

    let storage_ref = state.get_existing_collection(&collection)?;
    let lock_start = Instant::now();
    let storage = storage_ref.read();
    record_lock_read(state.registry.tracker(&collection).as_deref(), lock_start);

    Ok(CountResponse {
        count: storage.count(),
    })
}

pub fn index_stats(state: &SharedState, collection: String) -> Result<IndexStatsResponse> {
    ensure_available(state)?;

    let storage_ref = state.get_existing_collection(&collection)?;
    let lock_start = Instant::now();
    let storage = storage_ref.read();
    record_lock_read(state.registry.tracker(&collection).as_deref(), lock_start);

    let stats = storage.vector_index().stats();
    Ok(IndexStatsResponse {
        index_type: stats.index_type.to_string(),
        total_vectors: stats.total_vectors,
        memory_usage_bytes: stats.memory_usage_bytes,
        details: serde_json::to_value(&stats.details).unwrap_or(serde_json::json!({})),
    })
}

pub fn rebuild_index(state: &SharedState, collection: String) -> Result<RebuildIndexResponse> {
    ensure_available(state)?;

    let storage_ref = state.get_existing_collection(&collection)?;
    let started_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    state.rebuild_jobs.insert(
        collection.clone(),
        RebuildJobStatus {
            status: RebuildState::Running,
            started_at,
            finished_at: None,
            error: None,
            elapsed_ms: None,
        },
    );

    let collection_name = collection.clone();
    let storage_ref_clone = storage_ref.clone();
    let jobs = state.rebuild_jobs.clone();

    tokio::task::spawn_blocking(move || {
        let mut storage = storage_ref_clone.write();
        let start = Instant::now();
        if let Err(e) = storage.rebuild_index() {
            tracing::error!(collection=%collection_name, error=%e, "index_rebuild_failed");
            let finished = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            jobs.insert(
                collection_name.clone(),
                RebuildJobStatus {
                    status: RebuildState::Failed,
                    started_at,
                    finished_at: Some(finished),
                    error: Some(e.to_string()),
                    elapsed_ms: Some(start.elapsed().as_millis()),
                },
            );
        } else {
            tracing::info!(
                collection=%collection_name,
                elapsed_ms = start.elapsed().as_millis(),
                "index_rebuild_complete"
            );
            let finished = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            jobs.insert(
                collection_name.clone(),
                RebuildJobStatus {
                    status: RebuildState::Completed,
                    started_at,
                    finished_at: Some(finished),
                    error: None,
                    elapsed_ms: Some(start.elapsed().as_millis()),
                },
            );
        }
    });

    Ok(RebuildIndexResponse {
        success: true,
        latency_ms: None,
    })
}

pub fn find_duplicates(
    state: &SharedState,
    collection: String,
    req: DuplicateRequest,
) -> Result<DuplicateResponse> {
    ensure_available(state)?;

    let storage_ref = state.get_existing_collection(&collection)?;
    let lock_start = Instant::now();
    let storage = storage_ref.read();
    record_lock_read(state.registry.tracker(&collection).as_deref(), lock_start);

    let metric = match req.metric.as_deref() {
        Some("euclidean") => Metric::Euclidean,
        Some("dot") | Some("dot_product") => Metric::DotProduct,
        _ => Metric::Cosine,
    };
    let hits = crate::storage::collection::find_duplicates(
        &storage,
        metric,
        req.threshold,
        req.limit,
        req.k,
        req.ef,
        req.nprobe,
    )?;

    let pairs = hits
        .into_iter()
        .map(|hit| DuplicatePair {
            id_a: hit.id_a.to_string(),
            id_b: hit.id_b.to_string(),
            score: hit.score,
        })
        .collect();

    Ok(DuplicateResponse { pairs })
}

pub fn compact_collection(state: &SharedState, collection: String) -> Result<RebuildIndexResponse> {
    ensure_available(state)?;

    let storage_ref = state.get_existing_collection(&collection)?;
    let mut storage = storage_ref.write();
    let start = Instant::now();
    let stats = crate::storage::collection::compact(&mut storage)?;
    let duration = start.elapsed();
    tracing::info!(
        collection=%collection,
        original=stats.original_entries,
        compacted=stats.compacted_entries,
        elapsed_ms=duration.as_millis(),
        "collection_compacted"
    );

    Ok(RebuildIndexResponse {
        success: true,
        latency_ms: Some(duration.as_millis() as f32),
    })
}

pub fn rebuild_index_status(
    state: &SharedState,
    collection: String,
) -> Result<RebuildIndexStatusResponse> {
    ensure_available(state)?;

    let job = state
        .rebuild_jobs
        .get(&collection)
        .ok_or_else(|| ServerError::NotFound("No rebuild job found for this collection".into()))?;
    let status = match job.status {
        RebuildState::Running => "running",
        RebuildState::Completed => "completed",
        RebuildState::Failed => "failed",
    };
    Ok(RebuildIndexStatusResponse {
        status: status.to_string(),
        started_at: Some(job.started_at),
        finished_at: job.finished_at,
        elapsed_ms: job.elapsed_ms.map(|ms| ms as f32),
        error: job.error.clone(),
    })
}
