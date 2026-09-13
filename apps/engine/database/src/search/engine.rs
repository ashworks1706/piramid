//! Query execution: plans overfetch, asks the index for candidates, scores and filters them.

use crate::index::{IndexSearchRequest, MetadataReader, VectorIndex, VectorReader};
use piramid_core::config::SearchConfig;
use piramid_core::error::{IndexError, Result};
use piramid_core::metadata::Filter;
use piramid_core::Document;
use piramid_core::Hit;
use piramid_hardware::compute::{strategies::for_mode, ExecutionMode, Metric};
use uuid::Uuid;

/// Per-query overrides layered on top of the configured defaults of a collection.
#[derive(Debug, Clone, Copy)]
pub struct SearchParams<'a> {
    /// Backend to score with. Auto defers to the configured mode of the collection.
    pub mode: ExecutionMode,
    /// Metadata predicate. When present, the planner overfetches and post-filters.
    pub filter: Option<&'a Filter>,
    /// Multiplier applied to k when a filter is present, overriding the configured value.
    pub filter_overfetch_override: Option<usize>,
    /// Recall and speed knobs for this query, overriding the configured value.
    pub search_config_override: Option<SearchConfig>,
    /// Drop hits scoring below this. Applied before k truncates the result set.
    pub min_score: Option<f32>,
}

impl Default for SearchParams<'_> {
    fn default() -> Self {
        Self {
            mode: ExecutionMode::Auto,
            filter: None,
            filter_overfetch_override: None,
            search_config_override: None,
            min_score: None,
        }
    }
}

/// What a search runs against, as borrowed views.
pub struct SearchTarget<'a> {
    /// The ANN index to query.
    pub index: &'a dyn VectorIndex,
    /// Access to the vectors the index refers to.
    pub vectors: &'a dyn VectorReader,
    /// Access to per-document metadata, for filter evaluation.
    pub metadata: &'a dyn MetadataReader,
    /// Recall and speed defaults, used unless [SearchParams::search_config_override] is set.
    pub default_config: SearchConfig,
}

/// Run one query against a target.
pub fn search(
    target: &SearchTarget<'_>,
    query: &[f32],
    k: usize,
    metric: Metric,
    params: SearchParams<'_>,
    resolve: &(dyn Fn(&Uuid) -> Result<Option<Document>> + Sync),
) -> Result<Vec<Hit>> {
    let effective_search = params
        .search_config_override
        .unwrap_or(target.default_config);

    // A metadata filter or a score threshold applies after the index returns, so more than k
    // candidates are requested.
    let post_filtered = params.filter.is_some() || params.min_score.is_some();
    let base_overfetch = effective_search.filter_overfetch.max(1);
    let expansion = params
        .filter_overfetch_override
        .unwrap_or(base_overfetch)
        .max(1);
    let search_k = if post_filtered {
        k.saturating_mul(expansion)
    } else {
        k
    };

    let kernels = for_mode(params.mode)?;
    let neighbor_ids = target.index.search(
        IndexSearchRequest::new(
            query,
            search_k,
            target.vectors,
            effective_search,
            target.metadata,
        )
        .with_filter(params.filter),
    )?;

    let mut results = rescore(query, neighbor_ids, metric, kernels, resolve)?;

    if let Some(min_score) = params.min_score {
        results.retain(|hit| hit.score >= min_score);
    }
    if let Some(filter) = params.filter {
        results.retain(|hit| filter.matches(&hit.document.metadata));
    }
    // Scores are recomputed here, so the order of the index is discarded.
    rank_top_k(&mut results, k);
    Ok(results)
}

/// Resolve the candidates of the index and score them against the query in one batch call.
///
/// The score is recomputed against the stored vector, whatever the index returned.
fn rescore(
    query: &[f32],
    ids: Vec<Uuid>,
    metric: Metric,
    kernels: &dyn piramid_hardware::compute::DistanceKernels,
    resolve: &dyn Fn(&Uuid) -> Result<Option<Document>>,
) -> Result<Vec<Hit>> {
    let mut documents = Vec::with_capacity(ids.len());
    for id in ids {
        documents.push(resolve(&id)?.ok_or_else(|| {
            IndexError::SearchFailed(format!("index returned missing document {id}"))
        })?);
    }
    let Some(dim) = documents.first().map(|document| document.vector().len()) else {
        return Ok(Vec::new());
    };
    // A candidate of a different width is an error.
    let mut block = Vec::with_capacity(documents.len() * dim);
    for document in &documents {
        if document.vector().len() != dim {
            return Err(IndexError::SearchFailed(format!(
                "document {} is {} dimensions where the collection is {dim}",
                document.id,
                document.vector().len()
            ))
            .into());
        }
        block.extend_from_slice(document.vector());
    }
    let mut scores = vec![0.0; documents.len()];
    metric
        .calculate_batch(query, &block, dim, &mut scores, kernels)
        .map_err(|e| IndexError::SearchFailed(e.to_string()))?;
    Ok(documents
        .into_iter()
        .zip(scores)
        .map(|(document, score)| Hit { score, document })
        .collect())
}

/// Run several queries against a target, optionally in parallel.
pub fn search_batch(
    target: &SearchTarget<'_>,
    queries: &[Vec<f32>],
    k: usize,
    metric: Metric,
    params: SearchParams<'_>,
    parallel: bool,
    resolve: &(dyn Fn(&Uuid) -> Result<Option<Document>> + Sync),
) -> Result<Vec<Vec<Hit>>> {
    if parallel {
        use rayon::prelude::*;
        queries
            .par_iter()
            .map(|query| search(target, query, k, metric, params, resolve))
            .collect()
    } else {
        queries
            .iter()
            .map(|query| search(target, query, k, metric, params, resolve))
            .collect()
    }
}

/// Sort by score descending and keep the top k.
fn rank_top_k(results: &mut Vec<Hit>, k: usize) {
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(k);
}
