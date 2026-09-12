//! Reading stored documents by id or by page.

use std::time::Instant;

use uuid::Uuid;

use crate::services::api::{ListVectorsQuery, VectorResponse};
use crate::services::convert::metadata_to_json;
use crate::services::VECTOR_NOT_FOUND;
use crate::state::SharedState;
use piramid_core::error::{Result, ServerError};
use piramid_core::stats::record_lock_read;

/// One document of an existing collection, by UUID.
pub fn get_vector(state: &SharedState, collection: String, id: String) -> Result<VectorResponse> {
    state.ensure_available()?;

    let collection_handle = state.get_existing_collection(&collection)?;
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| ServerError::InvalidRequest("Invalid UUID".to_string()))?;

    let lock_start = Instant::now();
    let collection_guard = collection_handle.read();
    record_lock_read(
        state.collection_manager.tracker(&collection).as_deref(),
        lock_start,
    );

    let entry = collection_guard
        .get(&uuid)?
        .ok_or(ServerError::NotFound(VECTOR_NOT_FOUND.to_string()))?;
    Ok(VectorResponse {
        id: entry.id.to_string(),
        vector: entry.vector().to_vec(),
        text: entry.text,
        metadata: metadata_to_json(&entry.metadata),
    })
}

/// One page of the documents of an existing collection, in unspecified order.
pub fn list_vectors(
    state: &SharedState,
    collection: String,
    params: ListVectorsQuery,
) -> Result<Vec<VectorResponse>> {
    state.ensure_available()?;

    let collection_handle = state.get_existing_collection(&collection)?;
    let lock_start = Instant::now();
    let collection_guard = collection_handle.read();
    record_lock_read(
        state.collection_manager.tracker(&collection).as_deref(),
        lock_start,
    );

    collection_guard
        .page(params.offset, params.limit)?
        .into_iter()
        .map(|entry| {
            Ok(VectorResponse {
                id: entry.id.to_string(),
                vector: entry.vector().to_vec(),
                text: entry.text,
                metadata: metadata_to_json(&entry.metadata),
            })
        })
        .collect()
}
