use axum::{extract::{Path, Query, State, Extension}, Json};
use uuid::Uuid;
use std::sync::atomic::Ordering;
use std::time::Instant;
use std::collections::HashMap;
use crate::{Metric, Document};
use crate::error::{Result, ServerError};
use crate::validation;
use crate::server::metrics::{record_lock_read, record_lock_write};
use crate::server::types::range::RangeSearchRequest;
use super::super::{
    state::SharedState,
    types::*,
    helpers::{json_to_metadata, metadata_to_json},
};

const MAX_BATCH_SIZE: usize = 10_000;

fn build_single_entry(mut req: InsertRequest) -> Result<Document> {
    let text = req.text.clone().ok_or_else(|| ServerError::InvalidRequest("text is required for single insert".to_string()))?;
    validation::validate_text(&text)?;
    let vector = req.vector.take().ok_or_else(|| ServerError::InvalidRequest("vector is required for single insert".to_string()))?;
    validation::validate_vector(&vector)?;
    let mut vec_to_store = vector;
    if req.normalize {
        vec_to_store = validation::normalize_vector(&vec_to_store);
    }
    let metadata = json_to_metadata(req.metadata);
    Ok(Document::with_metadata(vec_to_store, text, metadata))
}

fn build_batch_entries(mut req: InsertRequest) -> Result<Vec<Document>> {
    let vectors = req.vectors.take().ok_or_else(|| ServerError::InvalidRequest("vectors are required for batch insert".to_string()))?;
    let texts = req.texts.clone().ok_or_else(|| ServerError::InvalidRequest("texts are required for batch insert".to_string()))?;
    validation::validate_batch_size(vectors.len(), MAX_BATCH_SIZE, "Insert")?;
    if vectors.len() != texts.len() {
        return Err(ServerError::InvalidRequest("vectors and texts length mismatch".to_string()).into());
    }
    validation::validate_vectors(&vectors)?;
    for t in &texts {
        validation::validate_text(t)?;
    }
    let vectors = if req.normalize {
        vectors.iter().map(|v| validation::normalize_vector(v)).collect()
    } else {
        vectors
    };

    let mut entries = Vec::with_capacity(vectors.len());
    for (idx, vector) in vectors.into_iter().enumerate() {
        let md = if idx < req.metadata_list.len() {
            json_to_metadata(req.metadata_list[idx].clone())
        } else {
            json_to_metadata(HashMap::new())
        };
        let entry = Document::with_metadata(
            vector,
            texts[idx].clone(),
            md,
        );
        entries.push(entry);
    }
    Ok(entries)
}

// Parse similarity metric from string
fn parse_metric(s: Option<String>) -> Metric {
    match s.as_deref() {
        Some("euclidean") => Metric::Euclidean,
        Some("dot") | Some("dot_product") => Metric::DotProduct,
        _ => Metric::Cosine,
    }
}

pub(crate) fn apply_search_overrides(base: crate::config::SearchConfig, req_ef: Option<usize>, req_nprobe: Option<usize>, req_overfetch: Option<usize>, preset: Option<String>) -> crate::config::SearchConfig {
    let mut cfg = base;
    // Apply preset first
    if let Some(p) = preset {
        match p.to_lowercase().as_str() {
            "fast" => {
                cfg.ef = Some(50);
                cfg.nprobe = Some(1);
            }
            "high" => {
                cfg.ef = Some(400);
                cfg.nprobe = Some(20);
            }
            _ => {} // balanced/default
        }
    }
    if let Some(ef) = req_ef {
        cfg.ef = Some(ef);
    }
    if let Some(nprobe) = req_nprobe {
        cfg.nprobe = Some(nprobe);
    }
    if let Some(of) = req_overfetch {
        cfg.filter_overfetch = of.max(1);
    }
    cfg
}

// POST /api/collections/:collection/vectors - store a new vector
pub async fn insert_vector(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Json(mut req): Json<InsertRequest>,
) -> Result<Json<InsertResultsResponse>> {
    if state.shutting_down.load(Ordering::Relaxed) {
        return Err(ServerError::ServiceUnavailable("Server is shutting down".to_string()).into());
    }

    // Validate inputs
    validation::validate_collection_name(&collection)?;

    state.get_or_create_collection(&collection)?;
    
    let storage_ref = state.collections.get(&collection)
        .ok_or_else(|| ServerError::NotFound(super::super::helpers::COLLECTION_NOT_FOUND.to_string()))?;
    let lock_start = Instant::now();
    let mut storage = storage_ref.write();
    record_lock_write(state.latency_tracker.get(&collection).as_deref(), lock_start);
    
    let response = match (req.vector.take(), req.vectors.take()) {
        (Some(vector), None) => {
            req.vector = Some(vector);
            let entry = build_single_entry(req)?;
            
            let start = Instant::now();
            let id = storage.insert(entry)?;
            let duration = start.elapsed();
            
            if let Some(tracker) = state.latency_tracker.get(&collection) {
                tracker.record_insert(duration);
            }
            
            InsertResultsResponse::Single(InsertResponse { 
                id: id.to_string(),
                latency_ms: Some(duration.as_millis() as f32),
            })
        }
        (None, Some(vectors)) => {
            req.vectors = Some(vectors);
            let texts_len = req.texts.as_ref().map(|t| t.len()).unwrap_or(0);
            let entries = build_batch_entries(req)?;

            let start = Instant::now();
            let ids: Vec<Uuid> = storage.insert_batch(entries)?;
            let duration = start.elapsed();
            
            if let Some(tracker) = state.latency_tracker.get(&collection) {
                tracker.record_insert(duration);
            }

            InsertResultsResponse::Multi(MultiInsertResponse { 
                ids: ids.into_iter().map(|id| id.to_string()).collect(),
                count: texts_len,
                latency_ms: Some(duration.as_millis() as f32),
            })
        }
        (Some(_), Some(_)) => return Err(ServerError::InvalidRequest("Provide either vector or vectors, not both".to_string()).into()),
        (None, None) => return Err(ServerError::InvalidRequest("No vectors provided".to_string()).into()),
    };
    
    Ok(Json(response))
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
    let lock_start = Instant::now();
    let storage = storage_ref.read();
    record_lock_read(state.latency_tracker.get(&collection).as_deref(), lock_start);
    
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
    let lock_start = Instant::now();
    let storage = storage_ref.read();
    record_lock_read(state.latency_tracker.get(&collection).as_deref(), lock_start);
    
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
) -> Result<Json<DeleteResultsResponse>> {
    if state.shutting_down.load(Ordering::Relaxed) {
        return Err(ServerError::ServiceUnavailable("Server is shutting down".to_string()).into());
    }

    state.get_or_create_collection(&collection)?;
    
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| ServerError::InvalidRequest("Invalid UUID".to_string()))?;
    
    let storage_ref = state.collections.get(&collection)
        .ok_or_else(|| ServerError::NotFound(super::super::helpers::COLLECTION_NOT_FOUND.to_string()))?;
    let mut storage = storage_ref.write();
    
    let start = Instant::now();
    let deleted = storage.delete(&uuid)?;
    let duration = start.elapsed();
    
    // Record latency
    if let Some(tracker) = state.latency_tracker.get(&collection) {
        tracker.record_delete(duration);
    }
    
    Ok(Json(DeleteResultsResponse::Single(DeleteResponse { 
        deleted,
        latency_ms: Some(duration.as_millis() as f32),
    })))
}

// DELETE /api/collections/:collection/vectors - delete multiple vectors at once
pub async fn delete_vectors(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Json(req): Json<DeleteVectorsRequest>,
) -> Result<Json<DeleteResultsResponse>> {
    if state.shutting_down.load(Ordering::Relaxed) {
        return Err(ServerError::ServiceUnavailable("Server is shutting down".to_string()).into());
    }

    validation::validate_collection_name(&collection)?;
    validation::validate_batch_size(req.ids.len(), MAX_BATCH_SIZE, "Delete")?;

    state.get_or_create_collection(&collection)?;

    let storage_ref = state.collections.get(&collection)
        .ok_or_else(|| ServerError::NotFound(super::super::helpers::COLLECTION_NOT_FOUND.to_string()))?;
    let lock_start = Instant::now();
    let mut storage = storage_ref.write();
    record_lock_write(state.latency_tracker.get(&collection).as_deref(), lock_start);

    let mut uuids = Vec::with_capacity(req.ids.len());
    for id_str in &req.ids {
        let uuid = Uuid::parse_str(id_str)
            .map_err(|_| ServerError::InvalidRequest(format!("Invalid UUID: {}", id_str)))?;
        uuids.push(uuid);
    }

    let start = Instant::now();
    let deleted_count = storage.delete_batch(&uuids)?;
    let duration = start.elapsed();

    if let Some(tracker) = state.latency_tracker.get(&collection) {
        tracker.record_delete(duration);
    }

    Ok(Json(DeleteResultsResponse::Multi(MultiDeleteResponse { 
        deleted_count,
        latency_ms: Some(duration.as_millis() as f32),
    })))
}

// POST /api/collections/:collection/search - search for similar vectors
pub async fn search_vectors(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Extension(request_id): Extension<crate::server::request_id::RequestId>,
    Json(req): Json<SearchRequest>,
) -> Result<Json<SearchResultsResponse>> {

    // 1. Check if server is shutting down and reject new search requests if so, to allow for graceful shutdown without accepting new work.
    if state.shutting_down.load(Ordering::Relaxed) {
        return Err(ServerError::ServiceUnavailable("Server is shutting down".to_string()).into());
    }

    // 2. Validate the collection name and search vectors in the request to ensure they meet the required formats and constraints before proceeding with the search operation.
    validation::validate_collection_name(&collection)?;

    state.get_or_create_collection(&collection)?;
    
    // 3. Acquire a read lock on the collection's storage to ensure thread-safe access while performing the search operation, and record the time taken to acquire the lock for latency tracking purposes.
    let storage_ref = state.collections.get(&collection)
        .ok_or_else(|| ServerError::NotFound(super::super::helpers::COLLECTION_NOT_FOUND.to_string()))?;
    let lock_start = Instant::now();
    let storage = storage_ref.read();
    record_lock_read(state.latency_tracker.get(&collection).as_deref(), lock_start);
    // 4. Parse the similarity metric and apply any search configuration overrides based on the request parameters, such as ef, nprobe, overfetch, or preset, to determine the effective search configuration that will be used for the search operation.
    let SearchRequest { vector, vectors, k, metric, ef, nprobe, overfetch, preset } = req;
    let metric = parse_metric(metric);
    let effective_search = apply_search_overrides(
        storage.config().search,
        ef,
        nprobe,
        overfetch,
        preset.clone(),
    );
    // 5. Perform the search operation using the storage's search method, passing in the search vector(s), k, metric, and effective search configuration. After obtaining the search results, filter them by min_score if it's a range search, and record the time taken for the search operation to track latency. If the search takes longer than a configured threshold, log a warning for slow queries.
    let response = match (vector, vectors) {
        (Some(vec), None) => {
            // 1. Validate the search vector to ensure it meets the required format and constraints before performing the search operation.
            validation::validate_vector(&vec)?;
            let start = Instant::now();
            // 2. Start storage search with the provided search vector, k, metric, and effective search configuration. The search method will return a list of results that are similar to the search vector based on the specified metric and search configuration.
            let results = storage.search(
                &vec,
                k,
                metric,
                crate::SearchParams {
                    mode: storage.config().execution,
                    filter: None,
                    filter_overfetch_override: overfetch,
                    search_config_override: Some(effective_search),
                },
            );
            // 3. If the search is a range search (indicated by the presence of min_score), filter the search results to include only those that meet the minimum score threshold, ensuring that the final results returned to the client are relevant based on the specified criteria.
            let duration = start.elapsed();
            if duration.as_millis() > state.slow_query_ms {
                tracing::warn!(
                    collection=%collection,
                    request_id = request_id.0.as_str(),
                    elapsed_ms = duration.as_millis(),
                    "slow_search"
                );
            }
            
            // 4. Record the latency of the search operation using the latency tracker associated with the collection, if available, to monitor and analyze search performance over time.
            if let Some(tracker) = state.latency_tracker.get(&collection) {
                tracker.record_search(duration);
            }


            // 5. Map the search results into the appropriate response format, including the ID, score, text, and metadata for each hit, and include the latency of the search operation in the response to provide insights into the performance of the search.

            let search_results: Vec<HitResponse> = results
                .into_iter()
                .map(|r| HitResponse {
                    id: r.id.to_string(),
                    score: r.score,
                    text: r.text,
                    metadata: metadata_to_json(&r.metadata),
                })
                .collect();
            
            // 6. Return the search results in a structured response format, including the list of hits and the latency of the search operation, to provide the client with the relevant information about the search results and the performance of the search.
            SearchResultsResponse::Single(SearchResponse { 
                results: search_results,
                latency_ms: Some(duration.as_millis() as f32),
            })
        }
        (None, Some(queries)) => {
            // 1. Validate the batch of search vectors to ensure they meet the required format and constraints before performing the batch search operation.
            validation::validate_batch_size(queries.len(), MAX_BATCH_SIZE, "Search")?;
            validation::validate_vectors(&queries)?;

            // 2. Start batch search with the provided search vectors, k, metric, and effective search configuration. The batch search method will return a list of results for each search vector, where each result is a list of hits that are similar to the corresponding search vector based on the specified metric and search configuration.

            let start = Instant::now();
            let params = crate::SearchParams {
                mode: storage.config().execution,
                filter: None,
                filter_overfetch_override: overfetch,
                search_config_override: Some(effective_search),
            };
            let batch_results = crate::search::search_batch_collection(
                &storage,
                &queries,
                k,
                metric,
                params,
            );
            let duration = start.elapsed();
            // 3. If the batch search takes longer than a configured threshold, log a warning for slow batch queries, including the collection name, request ID, and elapsed time in milliseconds to help identify and analyze performance issues with batch searches.
            if duration.as_millis() > state.slow_query_ms {
                tracing::warn!(
                    collection=%collection,
                    request_id = request_id.0.as_str(),
                    elapsed_ms = duration.as_millis(),
                    "slow_batch_search"
                );
            }

            if let Some(tracker) = state.latency_tracker.get(&collection) {
                tracker.record_search(duration);
            }


            // 4. Map the batch search results into the appropriate response format, where each search vector's results are represented as a list of hits with their ID, score, text, and metadata. Include the latency of the batch search operation in the response to provide insights into the performance of the batch search.
            let response_results: Vec<Vec<HitResponse>> = batch_results
                .into_iter()
                .map(|results: Vec<crate::search::Hit>| {
                    results
                        .into_iter()
                        .map(|r| HitResponse {
                            id: r.id.to_string(),
                            score: r.score,
                            text: r.text,
                            metadata: metadata_to_json(&r.metadata),
                        })
                        .collect()
                })
                .collect();

            // 5. Return the batch search results in a structured response format, where each entry corresponds to the results for a specific search vector, along with the latency of the batch search operation, to provide the client with comprehensive information about the batch search results and the performance of the batch search.

            SearchResultsResponse::Multi(MultiSearchResponse { 
                results: response_results,
                latency_ms: Some(duration.as_millis() as f32),
            })
        }
        (Some(_), Some(_)) => {
            return Err(ServerError::InvalidRequest("Provide either vector or vectors, not both".to_string()).into());
        }
        (None, None) => {
            return Err(ServerError::InvalidRequest("No search vector(s) provided".to_string()).into());
        }
    };
    
    Ok(Json(response))
}

// POST /api/collections/:collection/upsert - insert or update a vector
pub async fn upsert_vector(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Json(mut req): Json<UpsertRequest>,
) -> Result<Json<UpsertResponse>> {
    if state.shutting_down.load(Ordering::Relaxed) {
        return Err(ServerError::ServiceUnavailable("Server is shutting down".to_string()).into());
    }

    // Validate inputs
    validation::validate_collection_name(&collection)?;
    validation::validate_text(&req.text)?;
    validation::validate_vector(&req.vector)?;
    
    // Normalize if requested
    if req.normalize {
        req.vector = validation::normalize_vector(&req.vector);
    }

    state.get_or_create_collection(&collection)?;
    
    let storage_ref = state.collections.get(&collection)
        .ok_or_else(|| ServerError::NotFound(super::super::helpers::COLLECTION_NOT_FOUND.to_string()))?;
    let lock_start = Instant::now();
    let mut storage = storage_ref.write();
    record_lock_write(state.latency_tracker.get(&collection).as_deref(), lock_start);
    
    // Check if entry exists
    let id = if let Some(id_str) = req.id {
        Uuid::parse_str(&id_str)
            .map_err(|_| ServerError::InvalidRequest("Invalid UUID".to_string()))?
    } else {
        Uuid::new_v4()
    };
    
    let exists = storage.get(&id).is_some();
    
    let metadata = json_to_metadata(req.metadata);
    let mut entry = Document::with_metadata(req.vector, req.text, metadata);
    entry.id = id;
    
    let start = Instant::now();
    storage.upsert(entry)?;
    let duration = start.elapsed();
    
    // Record latency (treat as insert or update)
    if let Some(tracker) = state.latency_tracker.get(&collection) {
        if exists {
            tracker.record_update(duration);
        } else {
            tracker.record_insert(duration);
        }
    }
    
    Ok(Json(UpsertResponse { 
        id: id.to_string(),
        created: !exists,
        latency_ms: Some(duration.as_millis() as f32),
    }))
}

// POST /api/collections/:collection/search/range - search with a min_score threshold
pub async fn range_search_vectors(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Extension(request_id): Extension<crate::server::request_id::RequestId>,
    Json(req): Json<RangeSearchRequest>,
) -> Result<Json<SearchResponse>> {
    if state.shutting_down.load(Ordering::Relaxed) {
        return Err(ServerError::ServiceUnavailable("Server is shutting down".to_string()).into());
    }

    validation::validate_collection_name(&collection)?;
    validation::validate_vector(&req.vector)?;

    state.get_or_create_collection(&collection)?;

    let storage_ref = state.collections.get(&collection)
        .ok_or_else(|| ServerError::NotFound(super::super::helpers::COLLECTION_NOT_FOUND.to_string()))?;
    let lock_start = Instant::now();
    let storage = storage_ref.read();
    record_lock_read(state.latency_tracker.get(&collection).as_deref(), lock_start);

    let metric = parse_metric(req.metric);
    let effective_search = apply_search_overrides(
        storage.config().search,
        req.ef,
        req.nprobe,
        req.overfetch,
        req.preset.clone(),
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
    // Filter by min_score
    results.retain(|r| r.score >= req.min_score);
    let duration = start.elapsed();
    if duration.as_millis() > state.slow_query_ms {
        tracing::warn!(
            collection=%collection,
            request_id = request_id.0.as_str(),
            elapsed_ms = duration.as_millis(),
            "slow_range_search"
        );
    }

    let search_results: Vec<HitResponse> = results
        .into_iter()
        .map(|r| HitResponse {
            id: r.id.to_string(),
            score: r.score,
            text: r.text,
            metadata: metadata_to_json(&r.metadata),
        })
        .collect();

    Ok(Json(SearchResponse {
        results: search_results,
        latency_ms: Some(duration.as_millis() as f32),
    }))
}
