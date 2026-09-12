use std::time::Instant;

use crate::services::api::{RangeSearchRequest, SearchRequest, SearchResponse};
use crate::services::convert::{
    apply_search_overrides, hit_to_response, parse_filter, parse_metric,
};
use crate::state::SharedState;
use piramid_core::error::{Result, ServerError};
use piramid_core::stats::record_lock_read;
use piramid_core::validation;

use super::MAX_BATCH_SIZE;

/// Search a collection with one or more query vectors.
#[tracing::instrument(
    name = "search",
    target = "piramid::search",
    skip_all,
    fields(
        collection = %collection,
        request_id = request_id,
        k = req.k,
        batch = req.vectors.len(),
        index_type = tracing::field::Empty,
        ef = tracing::field::Empty,
        nprobe = tracing::field::Empty,
        results = tracing::field::Empty,
        elapsed_ms = tracing::field::Empty,
    )
)]
pub fn search_vectors(
    state: &SharedState,
    collection: String,
    request_id: &str,
    req: SearchRequest,
) -> Result<SearchResponse> {
    state.ensure_available()?;
    validation::validate_collection_name(&collection)?;

    let SearchRequest {
        vectors,
        k,
        metric,
        filter,
        tuning,
    } = req;
    if vectors.is_empty() {
        return Err(ServerError::InvalidRequest("vectors must not be empty".to_string()).into());
    }
    validation::validate_batch_size(vectors.len(), MAX_BATCH_SIZE, "Search")?;
    validation::validate_vectors(&vectors)?;

    let filter = parse_filter(filter)?;

    let collection_handle = state.get_existing_collection(&collection)?;
    let lock_start = Instant::now();
    let collection_guard = collection_handle.read();
    record_lock_read(
        state.collection_manager.tracker(&collection).as_deref(),
        lock_start,
    );
    let metric = parse_metric(metric, collection_guard.vector_index().metric())?;

    let effective_search = apply_search_overrides(collection_guard.config().search, &tuning)?;

    let span = tracing::Span::current();
    span.record(
        "index_type",
        tracing::field::display(collection_guard.vector_index().index_type()),
    );
    if let Some(ef) = effective_search.ef {
        span.record("ef", ef);
    }
    if let Some(nprobe) = effective_search.nprobe {
        span.record("nprobe", nprobe);
    }

    let params = piramid_database::search::SearchParams {
        mode: collection_guard.config().execution,
        filter: filter.as_ref(),
        filter_overfetch_override: tuning.filter_overfetch,
        search_config_override: Some(effective_search),
        min_score: None,
    };

    let start = Instant::now();
    let batch_results = collection_guard.search_batch_with(&vectors, k, metric, params)?;
    let duration = start.elapsed();

    if duration.as_millis() > state.slow_query_ms() {
        tracing::warn!(
            target: "piramid::search",
            collection=%collection,
            request_id = request_id,
            elapsed_ms = duration.as_millis(),
            "slow_search"
        );
    }
    if let Some(tracker) = state.collection_manager.tracker(&collection) {
        tracker.record_search(duration);
    }
    span.record("results", batch_results.iter().map(Vec::len).sum::<usize>());
    span.record("elapsed_ms", duration.as_millis() as u64);

    Ok(SearchResponse {
        results: batch_results
            .into_iter()
            .map(|results| results.into_iter().map(hit_to_response).collect())
            .collect(),
        latency_ms: duration.as_millis() as f32,
    })
}

/// Range search: nearest neighbours filtered to a minimum score.
#[tracing::instrument(
    name = "range_search",
    target = "piramid::search",
    skip_all,
    fields(
        collection = %collection,
        request_id = request_id,
        k = req.k,
        min_score = req.min_score,
        results = tracing::field::Empty,
        elapsed_ms = tracing::field::Empty,
    )
)]
pub fn range_search_vectors(
    state: &SharedState,
    collection: String,
    request_id: &str,
    req: RangeSearchRequest,
) -> Result<SearchResponse> {
    state.ensure_available()?;
    validation::validate_collection_name(&collection)?;

    let RangeSearchRequest {
        vectors,
        min_score,
        metric,
        k,
        filter,
        tuning,
    } = req;
    if vectors.is_empty() {
        return Err(ServerError::InvalidRequest("vectors must not be empty".to_string()).into());
    }
    validation::validate_batch_size(vectors.len(), MAX_BATCH_SIZE, "Search")?;
    validation::validate_vectors(&vectors)?;

    let filter = parse_filter(filter)?;

    let collection_handle = state.get_existing_collection(&collection)?;
    let lock_start = Instant::now();
    let collection_guard = collection_handle.read();
    record_lock_read(
        state.collection_manager.tracker(&collection).as_deref(),
        lock_start,
    );
    let metric = parse_metric(metric, collection_guard.vector_index().metric())?;

    let effective_search = apply_search_overrides(collection_guard.config().search, &tuning)?;
    let params = piramid_database::search::SearchParams {
        mode: collection_guard.config().execution,
        filter: filter.as_ref(),
        filter_overfetch_override: tuning.filter_overfetch,
        search_config_override: Some(effective_search),
        min_score: Some(min_score),
    };

    let start = Instant::now();
    let batch_results = collection_guard.search_batch_with(&vectors, k, metric, params)?;
    let duration = start.elapsed();

    if duration.as_millis() > state.slow_query_ms() {
        tracing::warn!(
            target: "piramid::search",
            collection=%collection,
            request_id = request_id,
            elapsed_ms = duration.as_millis(),
            "slow_range_search"
        );
    }
    if let Some(tracker) = state.collection_manager.tracker(&collection) {
        tracker.record_search(duration);
    }
    let span = tracing::Span::current();
    span.record("results", batch_results.iter().map(Vec::len).sum::<usize>());
    span.record("elapsed_ms", duration.as_millis() as u64);

    Ok(SearchResponse {
        results: batch_results
            .into_iter()
            .map(|results| results.into_iter().map(hit_to_response).collect())
            .collect(),
        latency_ms: duration.as_millis() as f32,
    })
}
