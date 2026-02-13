use axum::{extract::{Path, State, Extension}, response::Json};
use std::sync::atomic::Ordering;
use std::time::Instant;
use crate::{Metric, Document};
use crate::error::{Result, ServerError};
use crate::server::metrics::{record_lock_read, record_lock_write};
use super::super::{
    state::SharedState,
    types::*,
    helpers::{json_to_metadata, metadata_to_json},
};

// Embedding endpoints: embed text then reuse storage/search flows

// Parse similarity metric from string
fn parse_metric(s: Option<String>) -> Metric {
    match s.as_deref() {
        Some("euclidean") => Metric::Euclidean,
        Some("dot") | Some("dot_product") => Metric::DotProduct,
        _ => Metric::Cosine,
    }
}

// POST /api/collections/:collection/embed - embed text and store
pub async fn embed_text(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Json(req): Json<EmbedRequest>,
) -> Result<Json<EmbedResponse>> {
    if state.shutting_down.load(Ordering::Relaxed) {
        return Err(ServerError::ServiceUnavailable("Server is shutting down".to_string()).into());
    }

    state.get_or_create_collection(&collection)?;

    let embedder = state.embedder.as_ref()
        .ok_or(ServerError::ServiceUnavailable(super::super::helpers::EMBEDDING_NOT_CONFIGURED.to_string()))?;

    let response = embedder.embed(&req.text).await?;

    let metadata = json_to_metadata(req.metadata);
    let entry = Document::with_metadata(
        response.embedding.clone(),
        req.text,
        metadata,
    );

    let storage_ref = state.collections.get(&collection)
        .ok_or_else(|| ServerError::NotFound(super::super::helpers::COLLECTION_NOT_FOUND.to_string()))?;
    let lock_start = Instant::now();
    let mut storage = storage_ref.write();
    record_lock_write(state.latency_tracker.get(&collection).as_deref(), lock_start);

    let id = storage.insert(entry)?;

    Ok(Json(EmbedResponse {
        id: id.to_string(),
        embedding: response.embedding,
        tokens: response.tokens,
    }))
}

// POST /api/collections/:collection/embed/batch - batch embed texts
pub async fn embed_batch(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Json(req): Json<EmbedBatchRequest>,
) -> Result<Json<EmbedBatchResponse>> {
    if state.shutting_down.load(Ordering::Relaxed) {
        return Err(ServerError::ServiceUnavailable("Server is shutting down".to_string()).into());
    }

    if req.texts.is_empty() {
        return Err(ServerError::InvalidRequest("No texts provided".to_string()).into());
    }

    state.get_or_create_collection(&collection)?;

    let embedder = state.embedder.as_ref()
        .ok_or(ServerError::ServiceUnavailable(super::super::helpers::EMBEDDING_NOT_CONFIGURED.to_string()))?;

    let responses: Vec<crate::embeddings::EmbeddingResponse> = embedder.embed_batch(&req.texts).await?;

    let storage_ref = state.collections.get(&collection)
        .ok_or_else(|| ServerError::NotFound(super::super::helpers::COLLECTION_NOT_FOUND.to_string()))?;
    let lock_start = Instant::now();
    let mut storage = storage_ref.write();
    record_lock_write(state.latency_tracker.get(&collection).as_deref(), lock_start);

    let mut ids = Vec::with_capacity(responses.len());
    let mut total_tokens = 0u32;

    for (idx, response) in responses.into_iter().enumerate() {
        let metadata = if idx < req.metadata.len() {
            json_to_metadata(req.metadata[idx].clone())
        } else {
            crate::Metadata::new()
        };

        let entry = Document::with_metadata(
            response.embedding,
            req.texts[idx].clone(),
            metadata,
        );

        let id = storage.insert(entry)?;

        ids.push(id.to_string());
        if let Some(tokens) = response.tokens {
            total_tokens += tokens;
        }
    }

    Ok(Json(EmbedBatchResponse {
        ids,
        total_tokens: if total_tokens > 0 { Some(total_tokens) } else { None },
    }))
}

// POST /api/collections/:collection/search/text - search by text query
pub async fn search_by_text(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Extension(request_id): Extension<crate::server::request_id::RequestId>,
    Json(req): Json<TextSearchRequest>,
) -> Result<Json<SearchResponse>> {
    if state.shutting_down.load(Ordering::Relaxed) {
        return Err(ServerError::ServiceUnavailable("Server is shutting down".to_string()).into());
    }

    state.get_or_create_collection(&collection)?;

    let embedder = state.embedder.as_ref()
        .ok_or(ServerError::ServiceUnavailable(super::super::helpers::EMBEDDING_NOT_CONFIGURED.to_string()))?;

    let response = embedder.embed(&req.query).await?;

    let metric = parse_metric(req.metric);
    let effective_search = crate::server::handlers::vectors::apply_search_overrides(
        state.collections.get(&collection)
            .map(|c| c.read().config().search)
            .unwrap_or_default(),
        req.ef,
        req.nprobe,
        req.overfetch,
        req.preset.clone(),
    );

    let storage_ref = state.collections.get(&collection)
        .ok_or_else(|| ServerError::NotFound(super::super::helpers::COLLECTION_NOT_FOUND.to_string()))?;
    let lock_start = Instant::now();
    let storage = storage_ref.read();
    record_lock_read(state.latency_tracker.get(&collection).as_deref(), lock_start);

    let start = Instant::now();
    let results: Vec<HitResponse> = storage
        .search(
            &response.embedding,
            req.k,
            metric,
            crate::SearchParams {
                mode: storage.config().execution,
                filter: None,
                filter_overfetch_override: req.overfetch,
                search_config_override: Some(effective_search),
            },
        )
        .into_iter()
        .map(|r| HitResponse {
            id: r.id.to_string(),
            score: r.score,
            text: r.text,
            metadata: metadata_to_json(&r.metadata),
        })
        .collect();
    let duration = start.elapsed();
    if duration.as_millis() > state.slow_query_ms {
        tracing::warn!(
            collection=%collection,
            request_id = request_id.0.as_str(),
            elapsed_ms = duration.as_millis(),
            "slow_text_search"
        );
    }
    
    // Record latency
    if let Some(tracker) = state.latency_tracker.get(&collection) {
        tracker.record_search(duration);
    }

    Ok(Json(SearchResponse { 
        results,
        latency_ms: Some(duration.as_millis() as f32),
    }))
}
