//! Finding pairs of documents that are near-copies of each other.
//!
//! An all-pairs neighbour scan: ask the index for the neighbours of each document, score each
//! pair once, and keep the pairs at or above a threshold. Every stored vector is a query.

use std::collections::HashSet;

use piramid_core::config::SearchConfig;
use piramid_core::error::Result;
use piramid_hardware::compute::{strategies::for_mode, ExecutionMode, Metric};
use uuid::Uuid;

use crate::index::IndexSearchRequest;
use crate::search::SearchTarget;

/// Two documents alike enough to be worth reporting, and how alike.
#[derive(Debug)]
pub struct DuplicatePair {
    /// One document of the pair.
    pub id_a: Uuid,
    /// The other document of the pair.
    pub id_b: Uuid,
    /// Similarity between the two, higher is closer.
    pub score: f32,
}

/// What a duplicate scan asks for.
pub struct DuplicateParams {
    /// Report pairs scoring at or above this.
    pub threshold: f32,
    /// Neighbours examined per document. Counts the hit of the document itself, so a value of n
    /// compares against at most n - 1 others.
    pub neighbors: usize,
    /// Stop after this many pairs, highest score first. None reports all of them.
    pub limit: Option<usize>,
    /// Recall knobs for the traversal, overriding the defaults of the target.
    pub search_config_override: Option<SearchConfig>,
}

/// Scan a target for pairs of near-identical documents.
pub fn near_duplicates(
    target: &SearchTarget<'_>,
    metric: Metric,
    mode: ExecutionMode,
    params: DuplicateParams,
) -> Result<Vec<DuplicatePair>> {
    let kernels = for_mode(mode)?;
    let search_config = params
        .search_config_override
        .unwrap_or(target.default_config);

    let ids: Vec<Uuid> = target.vectors.iter().map(|(id, _)| id).collect();
    let neighbors = params.neighbors.min(ids.len().saturating_sub(1)).max(1);

    let mut seen = HashSet::new();
    let mut pairs = Vec::new();

    for id in &ids {
        let Some(vector) = target.vectors.get(id) else {
            continue;
        };
        let found = target.index.search(IndexSearchRequest::new(
            vector,
            neighbors,
            target.vectors,
            search_config,
            target.metadata,
        ))?;

        for neighbor in found {
            if neighbor == *id {
                continue;
            }
            // Pairs are keyed in canonical order and scored once.
            let pair = if id < &neighbor {
                (*id, neighbor)
            } else {
                (neighbor, *id)
            };
            if !seen.insert(pair) {
                continue;
            }
            let (Some(a), Some(b)) = (target.vectors.get(&pair.0), target.vectors.get(&pair.1))
            else {
                continue;
            };
            let score = metric.calculate(a, b, kernels);
            if score >= params.threshold {
                pairs.push(DuplicatePair {
                    id_a: pair.0,
                    id_b: pair.1,
                    score,
                });
            }
        }
    }

    pairs.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    if let Some(max) = params.limit {
        pairs.truncate(max);
    }
    Ok(pairs)
}
