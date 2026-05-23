use std::collections::HashMap;
use std::time::Instant;

use crate::error::{Result, ServerError};
use crate::server::helpers::{json_to_metadata, EMBEDDING_NOT_CONFIGURED};
use crate::server::metrics::{record_lock_read, record_lock_write};
use crate::server::request_id::RequestId;
use crate::server::state::SharedState;
use crate::server::types::*;
use crate::services::search::{apply_search_overrides, hit_to_response, parse_metric};
use crate::Document;

fn ensure_available(state: &SharedState) -> Result<()> {
    if state
        .shutting_down
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        return Err(ServerError::ServiceUnavailable("Server is shutting down".to_string()).into());
    }
    Ok(())
}

pub async fn embed_text(
    state: &SharedState,
    collection: String,
    req: EmbedRequest,
) -> Result<EmbedResultsResponse> {
    ensure_available(state)?;
    state.ensure_write_allowed()?;

    let storage_ref = state.get_or_create_collection(&collection)?;
    let embedder = state
        .embedder
        .as_ref()
        .ok_or(ServerError::ServiceUnavailable(
            EMBEDDING_NOT_CONFIGURED.to_string(),
        ))?;

    match (req.text.clone(), req.texts.clone()) {
        (Some(text), None) => {
            tracing::info!(collection=%collection, "embed_single_request");
            let start = Instant::now();
            let response = embedder.embed(&text).await?;
            let embed_duration = start.elapsed();

            let lock_start = Instant::now();
            let mut storage = storage_ref.write();
            record_lock_write(state.registry.tracker(&collection).as_deref(), lock_start);

            let entry = Document::with_metadata(
                response.embedding.clone(),
                text,
                json_to_metadata(req.metadata),
            );
            let id = storage.insert(entry)?;
            state.enforce_cache_budget();
            state
                .embed_metrics
                .record(1, 1, response.tokens.unwrap_or(0) as u64, embed_duration);

            Ok(EmbedResultsResponse::Single(EmbedResponse {
                id: id.to_string(),
                embedding: response.embedding,
                tokens: response.tokens,
            }))
        }
        (None, Some(texts)) => {
            if texts.is_empty() {
                return Err(
                    ServerError::InvalidRequest("texts cannot be empty".to_string()).into(),
                );
            }
            tracing::info!(collection=%collection, batch=texts.len(), "embed_batch_request");

            let mut ids = Vec::with_capacity(texts.len());
            let mut embeddings = Vec::with_capacity(texts.len());
            let mut total_tokens: u32 = 0;
            let mut entries = Vec::with_capacity(texts.len());
            let start = Instant::now();
            for (idx, text) in texts.iter().enumerate() {
                let response = embedder.embed(text).await?;
                embeddings.push(response.embedding.clone());
                if let Some(tokens) = response.tokens {
                    total_tokens = total_tokens.saturating_add(tokens);
                }
                let metadata = if idx < req.metadata_list.len() {
                    json_to_metadata(req.metadata_list[idx].clone())
                } else {
                    json_to_metadata(HashMap::new())
                };
                entries.push(Document::with_metadata(
                    response.embedding,
                    text.clone(),
                    metadata,
                ));
            }

            let lock_start = Instant::now();
            let mut storage = storage_ref.write();
            record_lock_write(state.registry.tracker(&collection).as_deref(), lock_start);

            let insert_ids = storage.insert_batch(entries)?;
            ids.extend(insert_ids.into_iter().map(|id| id.to_string()));
            state.enforce_cache_budget();
            state
                .embed_metrics
                .record(1, ids.len() as u64, total_tokens as u64, start.elapsed());

            Ok(EmbedResultsResponse::Multi(MultiEmbedResponse {
                ids,
                embeddings,
                total_tokens: if total_tokens > 0 {
                    Some(total_tokens)
                } else {
                    None
                },
            }))
        }
        (Some(_), Some(_)) => Err(ServerError::InvalidRequest(
            "Provide either text or texts, not both".to_string(),
        )
        .into()),
        (None, None) => Err(ServerError::InvalidRequest("No text provided".to_string()).into()),
    }
}

pub async fn search_by_text(
    state: &SharedState,
    collection: String,
    request_id: RequestId,
    req: TextSearchRequest,
) -> Result<SearchResponse> {
    ensure_available(state)?;

    let storage_ref = state.get_existing_collection(&collection)?;
    let embedder = state
        .embedder
        .as_ref()
        .ok_or(ServerError::ServiceUnavailable(
            EMBEDDING_NOT_CONFIGURED.to_string(),
        ))?;

    tracing::info!(collection=%collection, "search_by_text_request");
    let start = Instant::now();
    let response = embedder.embed(&req.query).await?;
    let embed_duration = start.elapsed();
    state
        .embed_metrics
        .record(1, 1, response.tokens.unwrap_or(0) as u64, embed_duration);

    let metric = parse_metric(req.metric);
    let base_search = {
        let storage = storage_ref.read();
        storage.config().search
    };
    let effective_search = apply_search_overrides(
        base_search,
        req.ef,
        req.nprobe,
        req.overfetch,
        req.preset.clone(),
    );

    let lock_start = Instant::now();
    let storage = storage_ref.read();
    record_lock_read(state.registry.tracker(&collection).as_deref(), lock_start);

    let start = Instant::now();
    let results = storage.search(
        &response.embedding,
        req.k,
        metric,
        crate::SearchParams {
            mode: storage.config().execution,
            filter: None,
            filter_overfetch_override: req.overfetch,
            search_config_override: Some(effective_search),
        },
    );
    let duration = start.elapsed();
    if duration.as_millis() > state.slow_query_ms {
        tracing::warn!(
            collection=%collection,
            request_id = request_id.0.as_str(),
            elapsed_ms = duration.as_millis(),
            "slow_text_search"
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
