//! Operations that call the configured embedding provider.

use std::time::Instant;

use crate::services::api::*;
use crate::services::convert::{
    apply_search_overrides, hit_to_response, json_to_metadata, parse_filter, parse_metric,
};
use crate::services::EMBEDDING_NOT_CONFIGURED;
use crate::state::SharedState;
use piramid_core::error::{Result, ServerError};
use piramid_core::metadata::Metadata;
use piramid_core::stats::{record_lock_read, record_lock_write};
use piramid_core::Document;

/// The configured embedder, or the 503 both embed endpoints answer without one.
fn require_embedder(
    state: &SharedState,
) -> Result<&std::sync::Arc<dyn piramid_model::embeddings::Embedder>> {
    state
        .embeddings
        .embedder()
        .ok_or_else(|| ServerError::ServiceUnavailable(EMBEDDING_NOT_CONFIGURED.to_string()).into())
}

/// Embed texts through the configured provider and store the resulting vectors.
#[tracing::instrument(
    name = "embed",
    target = "piramid::embeddings",
    skip_all,
    fields(collection = %collection, texts = req.texts.len())
)]
pub async fn embed_text(
    state: &SharedState,
    collection: String,
    req: EmbedRequest,
) -> Result<EmbedResponse> {
    state.ensure_available()?;
    state.ensure_write_allowed()?;

    let EmbedRequest { texts, metadata } = req;
    if texts.is_empty() {
        return Err(ServerError::InvalidRequest("texts must not be empty".to_string()).into());
    }
    // Metadata is either absent or one entry per text.
    if !metadata.is_empty() && metadata.len() != texts.len() {
        return Err(ServerError::InvalidRequest(format!(
            "metadata length mismatch: {} texts, {} metadata entries",
            texts.len(),
            metadata.len()
        ))
        .into());
    }

    let collection_handle = state.get_or_create_collection(&collection)?;
    let embedder = require_embedder(state)?;

    tracing::info!(
        target: "piramid::inference",
        collection=%collection,
        batch=texts.len(),
        "embed_request"
    );

    let mut metadata = metadata.into_iter();
    let mut embeddings = Vec::with_capacity(texts.len());
    let mut entries = Vec::with_capacity(texts.len());
    let mut total_tokens: u32 = 0;
    let start = Instant::now();
    for text in texts {
        let response = embedder.embed(&text).await?;
        embeddings.push(response.embedding.clone());
        if let Some(tokens) = response.tokens {
            total_tokens = total_tokens.saturating_add(tokens);
        }
        let metadata = match metadata.next() {
            Some(map) => json_to_metadata(map)?,
            None => Metadata::new(),
        };
        entries.push(Document::with_metadata(response.embedding, text, metadata));
    }

    let lock_start = Instant::now();
    let mut collection_guard = collection_handle.write();
    record_lock_write(
        state.collection_manager.tracker(&collection).as_deref(),
        lock_start,
    );

    let ids = collection_guard.insert_batch(entries)?;
    state.enforce_cache_budget();
    state.embeddings.metrics().record(
        1,
        ids.len() as u64,
        u64::from(total_tokens),
        start.elapsed(),
    );

    Ok(EmbedResponse {
        ids: ids.into_iter().map(|id| id.to_string()).collect(),
        embeddings,
        total_tokens: (total_tokens > 0).then_some(total_tokens),
    })
}

/// Embed a query string, then search with the resulting vector.
#[tracing::instrument(
    name = "search_by_text",
    target = "piramid::search",
    skip_all,
    fields(collection = %collection, request_id = request_id)
)]
pub async fn search_by_text(
    state: &SharedState,
    collection: String,
    request_id: &str,
    req: TextSearchRequest,
) -> Result<SearchResponse> {
    state.ensure_available()?;

    let collection_handle = state.get_existing_collection(&collection)?;
    let embedder = require_embedder(state)?;

    tracing::info!(
        target: "piramid::search",
        collection=%collection,
        "search_by_text_request"
    );
    let start = Instant::now();
    let response = embedder.embed(&req.query).await?;
    let embed_duration = start.elapsed();
    state.embeddings.metrics().record(
        1,
        1,
        u64::from(response.tokens.unwrap_or(0)),
        embed_duration,
    );

    let filter = parse_filter(req.filter)?;
    let base_search = {
        let collection_guard = collection_handle.read();
        collection_guard.config().search
    };
    let effective_search = apply_search_overrides(base_search, &req.tuning)?;

    let lock_start = Instant::now();
    let collection_guard = collection_handle.read();
    record_lock_read(
        state.collection_manager.tracker(&collection).as_deref(),
        lock_start,
    );
    let metric = parse_metric(req.metric, collection_guard.vector_index().metric())?;

    let start = Instant::now();
    let results = collection_guard.search(
        &response.embedding,
        req.k,
        metric,
        piramid_database::search::SearchParams {
            mode: collection_guard.config().execution,
            filter: filter.as_ref(),
            filter_overfetch_override: req.tuning.filter_overfetch,
            search_config_override: Some(effective_search),
            min_score: None,
        },
    )?;
    let duration = start.elapsed();
    if duration.as_millis() > state.slow_query_ms() {
        tracing::warn!(
            target: "piramid::search",
            collection=%collection,
            request_id = request_id,
            elapsed_ms = duration.as_millis(),
            "slow_text_search"
        );
    }
    if let Some(tracker) = state.collection_manager.tracker(&collection) {
        tracker.record_search(duration);
    }

    Ok(SearchResponse {
        results: vec![results.into_iter().map(hit_to_response).collect()],
        latency_ms: duration.as_millis() as f32,
    })
}
