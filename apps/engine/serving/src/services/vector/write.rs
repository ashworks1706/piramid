//! Inserting, upserting and deleting documents.

use std::time::Instant;

use uuid::Uuid;

use crate::services::api::{
    DeleteResponse, DeleteVectorsRequest, InsertRequest, InsertResponse, UpsertRequest,
    UpsertResponse,
};
use crate::services::convert::json_to_metadata;
use crate::state::SharedState;
use piramid_core::error::{Result, ServerError};
use piramid_core::metadata::Metadata;
use piramid_core::stats::record_lock_write;
use piramid_core::validation;
use piramid_core::Document;

use super::MAX_BATCH_SIZE;

/// Turn a request into documents, rejecting any length disagreement between its lists.
fn build_entries(req: InsertRequest) -> Result<Vec<Document>> {
    let InsertRequest {
        vectors,
        texts,
        metadata,
        normalize,
    } = req;

    validation::validate_batch_size(vectors.len(), MAX_BATCH_SIZE, "Insert")?;
    if vectors.is_empty() {
        return Err(ServerError::InvalidRequest("vectors must not be empty".to_string()).into());
    }
    if vectors.len() != texts.len() {
        return Err(ServerError::InvalidRequest(format!(
            "vectors and texts length mismatch: {} vectors, {} texts",
            vectors.len(),
            texts.len()
        ))
        .into());
    }
    // Metadata is either absent or one entry per vector.
    if !metadata.is_empty() && metadata.len() != vectors.len() {
        return Err(ServerError::InvalidRequest(format!(
            "metadata length mismatch: {} vectors, {} metadata entries",
            vectors.len(),
            metadata.len()
        ))
        .into());
    }

    validation::validate_vectors(&vectors)?;
    for text in &texts {
        validation::validate_text(text)?;
    }

    let mut metadata = metadata.into_iter();
    let mut entries = Vec::with_capacity(vectors.len());
    for (i, (vector, text)) in vectors.into_iter().zip(texts).enumerate() {
        let vector = if normalize {
            validation::normalize_vector(&vector)
                .map_err(|e| ServerError::InvalidRequest(format!("Vector at index {i}: {e}")))?
        } else {
            vector
        };
        let metadata = match metadata.next() {
            Some(map) => json_to_metadata(map)?,
            None => Metadata::new(),
        };
        entries.push(Document::with_metadata(vector, text, metadata));
    }
    Ok(entries)
}

/// Insert documents.
#[tracing::instrument(
    name = "insert",
    target = "piramid::writes",
    skip_all,
    fields(collection = %collection, inserted = tracing::field::Empty)
)]
pub fn insert_vector(
    state: &SharedState,
    collection: String,
    req: InsertRequest,
) -> Result<InsertResponse> {
    state.ensure_available()?;
    state.ensure_write_allowed()?;
    validation::validate_collection_name(&collection)?;

    let entries = build_entries(req)?;
    let collection_handle = state.get_or_create_collection(&collection)?;

    let lock_start = Instant::now();
    let mut collection_guard = collection_handle.write();
    record_lock_write(
        state.collection_manager.tracker(&collection).as_deref(),
        lock_start,
    );

    let start = Instant::now();
    let ids = collection_guard.insert_batch(entries)?;
    let duration = start.elapsed();

    if let Some(tracker) = state.collection_manager.tracker(&collection) {
        tracker.record_insert(duration);
    }
    tracing::Span::current().record("inserted", ids.len());

    Ok(InsertResponse {
        count: ids.len(),
        ids: ids.into_iter().map(|id| id.to_string()).collect(),
        latency_ms: duration.as_secs_f32() * 1000.0,
    })
}

/// Delete one document of an existing collection, by UUID.
pub fn delete_vector(
    state: &SharedState,
    collection: String,
    id: String,
) -> Result<DeleteResponse> {
    state.ensure_available()?;
    state.ensure_write_allowed()?;

    let collection_handle = state.get_existing_collection(&collection)?;
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| ServerError::InvalidRequest("Invalid UUID".to_string()))?;

    let lock_start = Instant::now();
    let mut collection_guard = collection_handle.write();
    record_lock_write(
        state.collection_manager.tracker(&collection).as_deref(),
        lock_start,
    );

    let start = Instant::now();
    let deleted = collection_guard.delete(&uuid)?;
    let duration = start.elapsed();

    Ok(DeleteResponse {
        deleted_count: usize::from(deleted),
        latency_ms: duration.as_secs_f32() * 1000.0,
    })
}

/// Delete several vectors by id.
#[tracing::instrument(
    name = "delete_vectors",
    target = "piramid::writes",
    skip_all,
    fields(collection = %collection)
)]
pub fn delete_vectors(
    state: &SharedState,
    collection: String,
    req: DeleteVectorsRequest,
) -> Result<DeleteResponse> {
    state.ensure_available()?;
    state.ensure_write_allowed()?;
    validation::validate_collection_name(&collection)?;
    validation::validate_batch_size(req.ids.len(), MAX_BATCH_SIZE, "Delete")?;

    let collection_handle = state.get_existing_collection(&collection)?;
    let mut uuids = Vec::with_capacity(req.ids.len());
    for id in &req.ids {
        let uuid = Uuid::parse_str(id)
            .map_err(|_| ServerError::InvalidRequest(format!("Invalid UUID: {id}")))?;
        uuids.push(uuid);
    }

    let lock_start = Instant::now();
    let mut collection_guard = collection_handle.write();
    record_lock_write(
        state.collection_manager.tracker(&collection).as_deref(),
        lock_start,
    );

    let start = Instant::now();
    let deleted_count = collection_guard.delete_batch(&uuids)?;
    let duration = start.elapsed();

    Ok(DeleteResponse {
        deleted_count,
        latency_ms: duration.as_secs_f32() * 1000.0,
    })
}

/// Insert a vector, replacing any existing one with the same id.
#[tracing::instrument(
    name = "upsert",
    target = "piramid::writes",
    skip_all,
    fields(collection = %collection)
)]
pub fn upsert_vector(
    state: &SharedState,
    collection: String,
    mut req: UpsertRequest,
) -> Result<UpsertResponse> {
    state.ensure_available()?;
    state.ensure_write_allowed()?;
    validation::validate_collection_name(&collection)?;
    validation::validate_text(&req.text)?;
    validation::validate_vector(&req.vector)?;

    if req.normalize {
        req.vector = validation::normalize_vector(&req.vector)?;
    }

    let collection_handle = state.get_or_create_collection(&collection)?;
    let lock_start = Instant::now();
    let mut collection_guard = collection_handle.write();
    record_lock_write(
        state.collection_manager.tracker(&collection).as_deref(),
        lock_start,
    );

    let id = if let Some(id) = req.id {
        Uuid::parse_str(&id).map_err(|_| ServerError::InvalidRequest("Invalid UUID".to_string()))?
    } else {
        Uuid::new_v4()
    };
    let exists = collection_guard.get(&id)?.is_some();
    let mut entry = Document::with_metadata(req.vector, req.text, json_to_metadata(req.metadata)?);
    entry.id = id;

    let start = Instant::now();
    collection_guard.upsert(entry)?;
    let duration = start.elapsed();

    if let (Some(tracker), false) = (state.collection_manager.tracker(&collection), exists) {
        tracker.record_insert(duration);
    }
    tracing::info!(
        target: "piramid::writes",
        collection=%collection,
        id=%id,
        created=!exists,
        "upsert_request"
    );

    Ok(UpsertResponse {
        id: id.to_string(),
        created: !exists,
        latency_ms: duration.as_secs_f32() * 1000.0,
    })
}
