//! Collection lifecycle, index maintenance and duplicate-scan operations.

use std::time::Instant;

use crate::services::api::*;
use crate::state::{RebuildJobStatus, RebuildState, SharedState};
use piramid_core::error::{Result, ServerError};
use piramid_core::stats::record_lock_read;
use piramid_core::validation;

fn collection_info(name: String, collection: &piramid_database::Collection) -> CollectionInfo {
    let meta = collection.manifest();
    CollectionInfo {
        name,
        count: collection.count(),
        created_at: Some(meta.created_at),
        updated_at: Some(meta.updated_at),
        dimensions: meta.dimensions,
    }
}

/// Summaries of the collections loaded in memory.
pub fn list_collections(state: &SharedState) -> Result<CollectionsResponse> {
    state.ensure_available()?;

    let mut collections = Vec::new();
    for (name, collection_handle) in state.collection_manager.loaded_collections() {
        let lock_start = Instant::now();
        let collection_guard = collection_handle.read();
        record_lock_read(
            state.collection_manager.tracker(&name).as_deref(),
            lock_start,
        );
        collections.push(collection_info(name, &collection_guard));
    }

    Ok(CollectionsResponse { collections })
}

/// Create a collection, or open it if it already exists, and return its summary.
pub fn create_collection(
    state: &SharedState,
    req: CreateCollectionRequest,
) -> Result<CollectionInfo> {
    state.ensure_available()?;
    validation::validate_collection_name(&req.name)?;

    let collection_handle = state.get_or_create_collection(&req.name)?;
    let lock_start = Instant::now();
    let collection_guard = collection_handle.read();
    record_lock_read(
        state.collection_manager.tracker(&req.name).as_deref(),
        lock_start,
    );
    Ok(collection_info(req.name, &collection_guard))
}

/// Summary of one existing collection, opening it from disk if needed.
pub fn get_collection(state: &SharedState, collection: String) -> Result<CollectionInfo> {
    state.ensure_available()?;

    let collection_handle = state.get_existing_collection(&collection)?;
    let lock_start = Instant::now();
    let collection_guard = collection_handle.read();
    record_lock_read(
        state.collection_manager.tracker(&collection).as_deref(),
        lock_start,
    );
    Ok(collection_info(collection, &collection_guard))
}

/// Unload a collection and remove its record file and sidecars.
pub fn delete_collection(
    state: &SharedState,
    collection: String,
) -> Result<DeleteCollectionResponse> {
    // Deleting frees disk space, so it is allowed while low disk space has writes disabled.
    state.ensure_available()?;
    state.collection_manager.delete(&collection)?;
    Ok(DeleteCollectionResponse { deleted: true })
}

/// Number of documents stored in one existing collection.
pub fn collection_count(state: &SharedState, collection: String) -> Result<CountResponse> {
    state.ensure_available()?;

    let collection_handle = state.get_existing_collection(&collection)?;
    let lock_start = Instant::now();
    let collection_guard = collection_handle.read();
    record_lock_read(
        state.collection_manager.tracker(&collection).as_deref(),
        lock_start,
    );

    Ok(CountResponse {
        count: collection_guard.count(),
    })
}

/// Statistics of the vector index of one existing collection.
pub fn index_stats(state: &SharedState, collection: String) -> Result<IndexStatsResponse> {
    state.ensure_available()?;

    let collection_handle = state.get_existing_collection(&collection)?;
    let lock_start = Instant::now();
    let collection_guard = collection_handle.read();
    record_lock_read(
        state.collection_manager.tracker(&collection).as_deref(),
        lock_start,
    );

    let stats = collection_guard.vector_index().stats();
    Ok(IndexStatsResponse {
        index_type: stats.index_type.to_string(),
        total_vectors: stats.total_vectors,
        memory_usage_bytes: stats.memory_usage_bytes,
        details: serde_json::to_value(&stats.details)?,
    })
}

/// Rebuild the ANN index of a collection from stored records.
#[tracing::instrument(
    name = "rebuild_index",
    target = "piramid::indexing",
    skip_all,
    fields(collection = %collection)
)]
pub fn rebuild_index(state: &SharedState, collection: String) -> Result<RebuildIndexResponse> {
    state.ensure_available()?;

    let collection_handle = state.get_existing_collection(&collection)?;
    let started_at = piramid_core::clock::unix_secs();
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
    let collection_handle_clone = collection_handle.clone();
    let jobs = state.rebuild_jobs.clone();

    tokio::task::spawn_blocking(move || {
        let mut collection_guard = collection_handle_clone.write();
        let start = Instant::now();
        if let Err(e) = collection_guard.rebuild_index() {
            tracing::error!(
                target: "piramid::indexing",
                collection=%collection_name,
                error=%e,
                "index_rebuild_failed"
            );
            let finished = piramid_core::clock::unix_secs();
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
                target: "piramid::indexing",
                collection=%collection_name,
                elapsed_ms = start.elapsed().as_millis(),
                "index_rebuild_complete"
            );
            let finished = piramid_core::clock::unix_secs();
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

/// Pairs of near-identical documents in one existing collection.
pub fn find_duplicates(
    state: &SharedState,
    collection: String,
    req: DuplicateRequest,
) -> Result<DuplicateResponse> {
    state.ensure_available()?;

    let collection_handle = state.get_existing_collection(&collection)?;
    let lock_start = Instant::now();
    let collection_guard = collection_handle.read();
    record_lock_read(
        state.collection_manager.tracker(&collection).as_deref(),
        lock_start,
    );

    let metric = crate::services::convert::parse_metric(
        req.metric,
        collection_guard.vector_index().metric(),
    )?;
    let hits = piramid_database::find_duplicates(
        &collection_guard,
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

/// Rewrite the record store of a collection, dropping dead entries.
#[tracing::instrument(
    name = "compact",
    target = "piramid::writes",
    skip_all,
    fields(collection = %collection)
)]
pub fn compact_collection(state: &SharedState, collection: String) -> Result<RebuildIndexResponse> {
    state.ensure_available()?;

    let collection_handle = state.get_existing_collection(&collection)?;
    let mut collection_guard = collection_handle.write();
    let start = Instant::now();
    let stats = piramid_database::compact(&mut collection_guard)?;
    let duration = start.elapsed();
    tracing::info!(
        target: "piramid::indexing",
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

/// State of the most recent index rebuild started for a collection.
pub fn rebuild_index_status(
    state: &SharedState,
    collection: String,
) -> Result<RebuildIndexStatusResponse> {
    state.ensure_available()?;

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
