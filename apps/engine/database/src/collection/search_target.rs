//! Collection-level search: checks a query against the collection, then runs it.

use crate::search::{SearchParams, SearchTarget};
use piramid_core::error::SearchError;
use piramid_core::Hit;
use piramid_core::Result;
use piramid_hardware::compute::{ExecutionMode, Metric};

use super::Collection;

pub(crate) fn target(collection: &Collection) -> SearchTarget<'_> {
    SearchTarget {
        vectors: collection.vector_reader(),
        metadata: collection.metadata_view(),
    }
}

/// Refuse a metric other than the one the collection was created with.
fn ensure_collection_metric(collection: &Collection, requested: Metric) -> Result<()> {
    let metric = collection.metric();
    if metric == requested {
        Ok(())
    } else {
        Err(SearchError::MetricMismatch {
            collection: metric,
            requested,
        }
        .into())
    }
}

/// Refuse a query the metric cannot score.
fn ensure_scorable(query: &[f32], metric: Metric) -> Result<()> {
    match metric {
        Metric::Cosine => piramid_core::validation::validate_cosine_magnitude(query),
        Metric::Euclidean | Metric::DotProduct => Ok(()),
    }
}

/// The params with an Auto mode replaced by the configured execution mode of the collection.
fn resolve_mode<'a>(collection: &Collection, mut params: SearchParams<'a>) -> SearchParams<'a> {
    if matches!(params.mode, ExecutionMode::Auto) {
        params.mode = collection.config().execution;
    }
    params
}

/// Search one query. An Auto mode scores with the configured execution mode of the collection.
pub fn search(
    collection: &Collection,
    query: &[f32],
    k: usize,
    metric: Metric,
    params: SearchParams,
) -> Result<Vec<Hit>> {
    ensure_collection_metric(collection, metric)?;
    ensure_scorable(query, metric)?;
    crate::search::search(
        &target(collection),
        query,
        k,
        metric,
        resolve_mode(collection, params),
        &|id| collection.get(id),
    )
}

/// Search many queries, in parallel when the search config of the collection allows.
pub fn search_batch(
    collection: &Collection,
    queries: &[Vec<f32>],
    k: usize,
    metric: Metric,
    params: SearchParams,
) -> Result<Vec<Vec<Hit>>> {
    ensure_collection_metric(collection, metric)?;
    for query in queries {
        ensure_scorable(query, metric)?;
    }
    crate::search::search_batch(
        &target(collection),
        queries,
        k,
        metric,
        resolve_mode(collection, params),
        collection.config().search.parallel,
        &|id| collection.get(id),
    )
}
