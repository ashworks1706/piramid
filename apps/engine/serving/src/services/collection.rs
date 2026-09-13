//! Collection lifecycle and compaction operations.

use std::time::Instant;

use crate::services::api::*;
use crate::state::SharedState;
use piramid_core::error::Result;
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
        metric: meta.metric.as_str().to_string(),
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
    // Deleting is allowed while low disk space has writes disabled.
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

/// Rewrite the record store of a collection, dropping dead entries.
#[tracing::instrument(
    name = "compact",
    target = "piramid::writes",
    skip_all,
    fields(collection = %collection)
)]
pub fn compact_collection(state: &SharedState, collection: String) -> Result<CompactResponse> {
    state.ensure_available()?;

    let collection_handle = state.get_existing_collection(&collection)?;
    let mut collection_guard = collection_handle.write();
    let start = Instant::now();
    let stats = piramid_database::compact(&mut collection_guard)?;
    let duration = start.elapsed();
    tracing::info!(
        target: "piramid::writes",
        collection=%collection,
        documents = stats.documents,
        bytes_before = stats.bytes_before,
        bytes_after = stats.bytes_after,
        elapsed_ms=duration.as_millis(),
        "collection_compacted"
    );

    Ok(CompactResponse {
        documents: stats.documents,
        bytes_before: stats.bytes_before,
        bytes_after: stats.bytes_after,
        latency_ms: duration.as_secs_f32() * 1000.0,
    })
}
