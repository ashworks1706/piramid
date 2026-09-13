//! Nearest-neighbour search over query vectors.

use std::time::Instant;

use crate::services::api::{SearchRequest, SearchResponse};
use crate::services::convert::{hit_to_response, parse_filter, parse_metric};
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
    let metric = parse_metric(metric, collection_guard.metric())?;

    let params = piramid_database::search::SearchParams {
        filter: filter.as_ref(),
        ..Default::default()
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
    let span = tracing::Span::current();
    span.record("results", batch_results.iter().map(Vec::len).sum::<usize>());
    span.record("elapsed_ms", duration.as_millis() as u64);

    Ok(SearchResponse {
        results: batch_results
            .into_iter()
            .map(|results| results.into_iter().map(hit_to_response).collect())
            .collect(),
        latency_ms: duration.as_secs_f32() * 1000.0,
    })
}
