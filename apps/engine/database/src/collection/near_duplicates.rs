//! Collection-level duplicate scan: adapts collection configuration into a search target.

use crate::search::{DuplicatePair, DuplicateParams};
use piramid_core::error::{Result, ServerError};
use piramid_hardware::compute::Metric;

use super::Collection;

/// Neighbours examined per document when the caller does not say.
///
/// Counts the hit of each document itself, so this compares against 48 others.
const DEFAULT_NEIGHBORS: usize = 49;

/// Scan a collection for near-identical pairs, filling unset knobs from its configuration.
#[allow(clippy::too_many_arguments)]
pub fn find_duplicates(
    collection: &Collection,
    metric: Metric,
    threshold: f32,
    limit: Option<usize>,
    neighbors_override: Option<usize>,
    ef_override: Option<usize>,
    nprobe_override: Option<usize>,
) -> Result<Vec<DuplicatePair>> {
    super::search_target::ensure_indexed_metric(collection, metric)?;
    for (value, name) in [
        (neighbors_override, "k"),
        (ef_override, "ef"),
        (nprobe_override, "nprobe"),
    ] {
        if value == Some(0) {
            return Err(ServerError::InvalidRequest(format!("{name} must be >= 1")).into());
        }
    }
    let mut search = collection.config.search;
    if let Some(ef) = ef_override {
        search.ef = Some(ef);
    }
    if let Some(nprobe) = nprobe_override {
        search.nprobe = Some(nprobe);
    }

    let live_ids: Vec<_> = collection.index.keys().copied().collect();
    crate::search::near_duplicates(
        &super::search_target::target(collection),
        &live_ids,
        metric,
        collection.config().execution,
        DuplicateParams {
            threshold,
            neighbors: neighbors_override.unwrap_or(DEFAULT_NEIGHBORS),
            limit,
            search_config_override: Some(search),
        },
    )
}
