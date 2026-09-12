//! Collection-level search: adapts collection configuration into a search target.

use crate::search::{SearchParams, SearchTarget};
use piramid_core::error::IndexError;
use piramid_core::Hit;
use piramid_core::Result;
use piramid_hardware::compute::{ExecutionMode, Metric};

use super::Collection;

pub(crate) fn target(collection: &Collection) -> SearchTarget<'_> {
    SearchTarget {
        index: collection.vector_index(),
        vectors: collection.vector_reader(),
        metadata: collection.metadata_view(),
        default_config: collection.config.search,
    }
}

/// Refuse a metric other than the one the index of the collection orders candidates by.
pub(crate) fn ensure_indexed_metric(collection: &Collection, requested: Metric) -> Result<()> {
    let indexed = collection.vector_index().metric();
    if indexed == requested {
        Ok(())
    } else {
        Err(IndexError::MetricMismatch { indexed, requested }.into())
    }
}

/// Search one query, filling unset params from the configuration of the collection.
pub fn search(
    collection: &Collection,
    query: &[f32],
    k: usize,
    metric: Metric,
    mut params: SearchParams,
) -> Result<Vec<Hit>> {
    ensure_indexed_metric(collection, metric)?;
    if matches!(params.mode, ExecutionMode::Auto) {
        params.mode = collection.config().execution;
    }
    if params.filter_overfetch_override.is_none() {
        params.filter_overfetch_override = Some(collection.config.search.filter_overfetch);
    }
    crate::search::search(&target(collection), query, k, metric, params, &|id| {
        collection.get(id)
    })
}

/// Search many queries, in parallel when the parallelism config of the collection allows.
pub fn search_batch(
    collection: &Collection,
    queries: &[Vec<f32>],
    k: usize,
    metric: Metric,
    params: SearchParams,
) -> Result<Vec<Vec<Hit>>> {
    ensure_indexed_metric(collection, metric)?;
    let mut params = params;
    if matches!(params.mode, ExecutionMode::Auto) {
        params.mode = collection.config().execution;
    }
    crate::search::search_batch(
        &target(collection),
        queries,
        k,
        metric,
        params,
        collection.config().search.parallel,
        &|id| collection.get(id),
    )
}
