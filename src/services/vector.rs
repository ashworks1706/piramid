use std::collections::HashMap;
use std::time::Instant;

use uuid::Uuid;

use crate::error::{Result, ServerError};
use crate::metrics::{record_lock_read, record_lock_write};
use crate::runtime::SharedState;
use crate::server::helpers::{json_to_metadata, metadata_to_json, VECTOR_NOT_FOUND};
use crate::server::request_id::RequestId;
use crate::server::types::range::RangeSearchRequest;
use crate::server::types::*;
use crate::services::search::{apply_search_overrides, hit_to_response, parse_metric};
use crate::validation;
use crate::Document;

const MAX_BATCH_SIZE: usize = 10_000;

fn ensure_available(state: &SharedState) -> Result<()> {
    if state
        .shutting_down
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        return Err(ServerError::ServiceUnavailable("Server is shutting down".to_string()).into());
    }
    Ok(())
}

fn build_single_entry(mut req: InsertRequest) -> Result<Document> {
    let text = req.text.clone().ok_or_else(|| {
        ServerError::InvalidRequest("text is required for single insert".to_string())
    })?;
    validation::validate_text(&text)?;
    let vector = req.vector.take().ok_or_else(|| {
        ServerError::InvalidRequest("vector is required for single insert".to_string())
    })?;
    validation::validate_vector(&vector)?;
    let vector = if req.normalize {
        validation::normalize_vector(&vector)
    } else {
        vector
    };
    Ok(Document::with_metadata(
        vector,
        text,
        json_to_metadata(req.metadata),
    ))
}

fn build_batch_entries(mut req: InsertRequest) -> Result<Vec<Document>> {
    let vectors = req.vectors.take().ok_or_else(|| {
        ServerError::InvalidRequest("vectors are required for batch insert".to_string())
    })?;
    let texts = req.texts.clone().ok_or_else(|| {
        ServerError::InvalidRequest("texts are required for batch insert".to_string())
    })?;
    validation::validate_batch_size(vectors.len(), MAX_BATCH_SIZE, "Insert")?;
    if vectors.len() != texts.len() {
        return Err(
            ServerError::InvalidRequest("vectors and texts length mismatch".to_string()).into(),
        );
    }
    validation::validate_vectors(&vectors)?;
    for text in &texts {
        validation::validate_text(text)?;
    }

    let vectors = if req.normalize {
        vectors
            .iter()
            .map(|vector| validation::normalize_vector(vector))
            .collect()
    } else {
        vectors
    };

    let mut entries = Vec::with_capacity(vectors.len());
    for (idx, vector) in vectors.into_iter().enumerate() {
        let metadata = if idx < req.metadata_list.len() {
            json_to_metadata(req.metadata_list[idx].clone())
        } else {
            json_to_metadata(HashMap::new())
        };
        entries.push(Document::with_metadata(
            vector,
            texts[idx].clone(),
            metadata,
        ));
    }
    Ok(entries)
}

pub fn insert_vector(
    state: &SharedState,
    collection: String,
    mut req: InsertRequest,
) -> Result<InsertResultsResponse> {
    ensure_available(state)?;
    state.ensure_write_allowed()?;
    validation::validate_collection_name(&collection)?;

    let storage_ref = state.get_or_create_collection(&collection)?;
    tracing::info!(
        collection=%collection,
        single=req.vector.is_some(),
        batch=req.vectors.as_ref().map(|vectors| vectors.len()),
        "insert_request"
    );

    let lock_start = Instant::now();
    let mut storage = storage_ref.write();
    record_lock_write(state.registry.tracker(&collection).as_deref(), lock_start);

    let response = match (req.vector.take(), req.vectors.take()) {
        (Some(vector), None) => {
            req.vector = Some(vector);
            let entry = build_single_entry(req)?;
            let start = Instant::now();
            let id = storage.insert(entry)?;
            let duration = start.elapsed();

            if let Some(tracker) = state.registry.tracker(&collection) {
                tracker.record_insert(duration);
            }
            state.enforce_cache_budget();

            InsertResultsResponse::Single(InsertResponse {
                id: id.to_string(),
                latency_ms: Some(duration.as_millis() as f32),
            })
        }
        (None, Some(vectors)) => {
            req.vectors = Some(vectors);
            let count = req.texts.as_ref().map(|texts| texts.len()).unwrap_or(0);
            let entries = build_batch_entries(req)?;
            let start = Instant::now();
            let ids = storage.insert_batch(entries)?;
            let duration = start.elapsed();

            if let Some(tracker) = state.registry.tracker(&collection) {
                tracker.record_insert(duration);
            }
            state.enforce_cache_budget();

            InsertResultsResponse::Multi(MultiInsertResponse {
                ids: ids.into_iter().map(|id| id.to_string()).collect(),
                count,
                latency_ms: Some(duration.as_millis() as f32),
            })
        }
        (Some(_), Some(_)) => {
            return Err(ServerError::InvalidRequest(
                "Provide either vector or vectors, not both".to_string(),
            )
            .into())
        }
        (None, None) => {
            return Err(ServerError::InvalidRequest("No vectors provided".to_string()).into())
        }
    };

    Ok(response)
}

pub fn get_vector(state: &SharedState, collection: String, id: String) -> Result<VectorResponse> {
    ensure_available(state)?;

    let storage_ref = state.get_existing_collection(&collection)?;
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| ServerError::InvalidRequest("Invalid UUID".to_string()))?;

    let lock_start = Instant::now();
    let storage = storage_ref.read();
    record_lock_read(state.registry.tracker(&collection).as_deref(), lock_start);

    let entry = storage
        .get(&uuid)
        .ok_or(ServerError::NotFound(VECTOR_NOT_FOUND.to_string()))?;
    Ok(VectorResponse {
        id: entry.id.to_string(),
        vector: entry.get_vector(),
        text: entry.text,
        metadata: metadata_to_json(&entry.metadata),
    })
}

pub fn list_vectors(
    state: &SharedState,
    collection: String,
    params: ListVectorsQuery,
) -> Result<Vec<VectorResponse>> {
    ensure_available(state)?;

    let storage_ref = state.get_existing_collection(&collection)?;
    let lock_start = Instant::now();
    let storage = storage_ref.read();
    record_lock_read(state.registry.tracker(&collection).as_deref(), lock_start);

    Ok(storage
        .get_all()
        .into_iter()
        .skip(params.offset)
        .take(params.limit)
        .map(|entry| VectorResponse {
            id: entry.id.to_string(),
            vector: entry.get_vector(),
            text: entry.text,
            metadata: metadata_to_json(&entry.metadata),
        })
        .collect())
}

pub fn delete_vector(
    state: &SharedState,
    collection: String,
    id: String,
) -> Result<DeleteResultsResponse> {
    ensure_available(state)?;
    state.ensure_write_allowed()?;

    let storage_ref = state.get_existing_collection(&collection)?;
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| ServerError::InvalidRequest("Invalid UUID".to_string()))?;

    let lock_start = Instant::now();
    let mut storage = storage_ref.write();
    record_lock_write(state.registry.tracker(&collection).as_deref(), lock_start);

    let start = Instant::now();
    let deleted = storage.delete(&uuid)?;
    let duration = start.elapsed();

    if let Some(tracker) = state.registry.tracker(&collection) {
        tracker.record_delete(duration);
    }

    Ok(DeleteResultsResponse::Single(DeleteResponse {
        deleted,
        latency_ms: Some(duration.as_millis() as f32),
    }))
}

pub fn delete_vectors(
    state: &SharedState,
    collection: String,
    req: DeleteVectorsRequest,
) -> Result<DeleteResultsResponse> {
    ensure_available(state)?;
    state.ensure_write_allowed()?;
    validation::validate_collection_name(&collection)?;
    validation::validate_batch_size(req.ids.len(), MAX_BATCH_SIZE, "Delete")?;

    let storage_ref = state.get_existing_collection(&collection)?;
    let mut uuids = Vec::with_capacity(req.ids.len());
    for id in &req.ids {
        let uuid = Uuid::parse_str(id)
            .map_err(|_| ServerError::InvalidRequest(format!("Invalid UUID: {}", id)))?;
        uuids.push(uuid);
    }

    let lock_start = Instant::now();
    let mut storage = storage_ref.write();
    record_lock_write(state.registry.tracker(&collection).as_deref(), lock_start);

    let start = Instant::now();
    let deleted_count = storage.delete_batch(&uuids)?;
    let duration = start.elapsed();

    if let Some(tracker) = state.registry.tracker(&collection) {
        tracker.record_delete(duration);
    }

    Ok(DeleteResultsResponse::Multi(MultiDeleteResponse {
        deleted_count,
        latency_ms: Some(duration.as_millis() as f32),
    }))
}

pub fn search_vectors(
    state: &SharedState,
    collection: String,
    request_id: RequestId,
    req: SearchRequest,
) -> Result<SearchResultsResponse> {
    ensure_available(state)?;
    validation::validate_collection_name(&collection)?;

    let storage_ref = state.get_existing_collection(&collection)?;
    let lock_start = Instant::now();
    let storage = storage_ref.read();
    record_lock_read(state.registry.tracker(&collection).as_deref(), lock_start);

    let SearchRequest {
        vector,
        vectors,
        k,
        metric,
        ef,
        nprobe,
        overfetch,
        preset,
    } = req;
    let metric = parse_metric(metric);
    let effective_search =
        apply_search_overrides(storage.config().search, ef, nprobe, overfetch, preset);

    match (vector, vectors) {
        (Some(vector), None) => {
            validation::validate_vector(&vector)?;
            let start = Instant::now();
            let results = storage.search(
                &vector,
                k,
                metric,
                crate::SearchParams {
                    mode: storage.config().execution,
                    filter: None,
                    filter_overfetch_override: overfetch,
                    search_config_override: Some(effective_search),
                },
            );
            let duration = start.elapsed();
            if duration.as_millis() > state.slow_query_ms {
                tracing::warn!(
                    collection=%collection,
                    request_id = request_id.0.as_str(),
                    elapsed_ms = duration.as_millis(),
                    "slow_search"
                );
            }
            if let Some(tracker) = state.registry.tracker(&collection) {
                tracker.record_search(duration);
            }

            Ok(SearchResultsResponse::Single(SearchResponse {
                results: results.into_iter().map(hit_to_response).collect(),
                latency_ms: Some(duration.as_millis() as f32),
            }))
        }
        (None, Some(queries)) => {
            validation::validate_batch_size(queries.len(), MAX_BATCH_SIZE, "Search")?;
            validation::validate_vectors(&queries)?;

            let start = Instant::now();
            let params = crate::SearchParams {
                mode: storage.config().execution,
                filter: None,
                filter_overfetch_override: overfetch,
                search_config_override: Some(effective_search),
            };
            let batch_results =
                crate::search::search_batch_collection(&storage, &queries, k, metric, params);
            let duration = start.elapsed();
            if duration.as_millis() > state.slow_query_ms {
                tracing::warn!(
                    collection=%collection,
                    request_id = request_id.0.as_str(),
                    elapsed_ms = duration.as_millis(),
                    "slow_batch_search"
                );
            }
            if let Some(tracker) = state.registry.tracker(&collection) {
                tracker.record_search(duration);
            }

            Ok(SearchResultsResponse::Multi(MultiSearchResponse {
                results: batch_results
                    .into_iter()
                    .map(|results| results.into_iter().map(hit_to_response).collect())
                    .collect(),
                latency_ms: Some(duration.as_millis() as f32),
            }))
        }
        (Some(_), Some(_)) => Err(ServerError::InvalidRequest(
            "Provide either vector or vectors, not both".to_string(),
        )
        .into()),
        (None, None) => {
            Err(ServerError::InvalidRequest("No search vector(s) provided".to_string()).into())
        }
    }
}

pub fn upsert_vector(
    state: &SharedState,
    collection: String,
    mut req: UpsertRequest,
) -> Result<UpsertResponse> {
    ensure_available(state)?;
    state.ensure_write_allowed()?;
    validation::validate_collection_name(&collection)?;
    validation::validate_text(&req.text)?;
    validation::validate_vector(&req.vector)?;

    if req.normalize {
        req.vector = validation::normalize_vector(&req.vector);
    }

    let storage_ref = state.get_or_create_collection(&collection)?;
    let lock_start = Instant::now();
    let mut storage = storage_ref.write();
    record_lock_write(state.registry.tracker(&collection).as_deref(), lock_start);

    let id = if let Some(id) = req.id {
        Uuid::parse_str(&id).map_err(|_| ServerError::InvalidRequest("Invalid UUID".to_string()))?
    } else {
        Uuid::new_v4()
    };
    let exists = storage.get(&id).is_some();
    let mut entry = Document::with_metadata(req.vector, req.text, json_to_metadata(req.metadata));
    entry.id = id;

    let start = Instant::now();
    storage.upsert(entry)?;
    let duration = start.elapsed();

    if let Some(tracker) = state.registry.tracker(&collection) {
        if exists {
            tracker.record_update(duration);
        } else {
            tracker.record_insert(duration);
        }
    }
    state.enforce_cache_budget();
    tracing::info!(
        collection=%collection,
        id=%id,
        created=!exists,
        "upsert_request"
    );

    Ok(UpsertResponse {
        id: id.to_string(),
        created: !exists,
        latency_ms: Some(duration.as_millis() as f32),
    })
}

pub fn range_search_vectors(
    state: &SharedState,
    collection: String,
    request_id: RequestId,
    req: RangeSearchRequest,
) -> Result<SearchResponse> {
    ensure_available(state)?;
    validation::validate_collection_name(&collection)?;
    validation::validate_vector(&req.vector)?;

    let storage_ref = state.get_existing_collection(&collection)?;
    let lock_start = Instant::now();
    let storage = storage_ref.read();
    record_lock_read(state.registry.tracker(&collection).as_deref(), lock_start);

    let metric = parse_metric(req.metric);
    let effective_search = apply_search_overrides(
        storage.config().search,
        req.ef,
        req.nprobe,
        req.overfetch,
        req.preset,
    );
    let start = Instant::now();
    let mut results = storage.search(
        &req.vector,
        req.k,
        metric,
        crate::SearchParams {
            mode: storage.config().execution,
            filter: None,
            filter_overfetch_override: req.overfetch,
            search_config_override: Some(effective_search),
        },
    );
    results.retain(|hit| hit.score >= req.min_score);
    let duration = start.elapsed();
    if duration.as_millis() > state.slow_query_ms {
        tracing::warn!(
            collection=%collection,
            request_id = request_id.0.as_str(),
            elapsed_ms = duration.as_millis(),
            "slow_range_search"
        );
    }
    if let Some(tracker) = state.registry.tracker(&collection) {
        tracker.record_search(duration);
    }

    Ok(SearchResponse {
        results: results.into_iter().map(hit_to_response).collect(),
        latency_ms: Some(duration.as_millis() as f32),
    })
}
