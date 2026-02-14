
use uuid::Uuid;
use std::collections::HashSet;

use crate::metrics::Metric;
use crate::error::Result;
use super::storage::Collection;

#[derive(Debug)]
pub struct DuplicateHit {
    pub id_a: Uuid,
    pub id_b: Uuid,
    pub score: f32,
}

/// Naive duplicate detection: compute pairwise similarity and return pairs above threshold.
/// Limit optionally caps the number of pairs returned.
pub fn find_duplicates(
    collection: &Collection,
    metric: Metric,
    threshold: f32,
    limit: Option<usize>,
    k_override: Option<usize>,
    ef_override: Option<usize>,
    nprobe_override: Option<usize>,
) -> Result<Vec<DuplicateHit>> {
    let mut pairs = Vec::new();
    let vectors = collection.vectors_view();
    let metadatas = collection.metadata_view();
    let ids: Vec<Uuid> = vectors.keys().cloned().collect();
    let mode = collection.config.execution;
    let mut search_cfg = collection.config.search;
    if let Some(ef) = ef_override {
        search_cfg.ef = Some(ef);
    }
    if let Some(nprobe) = nprobe_override {
        search_cfg.nprobe = Some(nprobe);
    }
    let k_default = 50usize.saturating_sub(1);
    let neighbor_k = k_override
        .or_else(|| limit.map(|l| l.saturating_mul(2).max(10)))
        .unwrap_or(k_default)
        .min(ids.len().saturating_sub(1))
        .max(1);

    let mut seen = HashSet::new();

    for id in &ids {
        let vec = match vectors.get(id) {
            Some(v) => v,
            None => continue,
        };
        let neighbors = collection.vector_index().search(
            vec,
            neighbor_k,
            vectors,
            search_cfg,
            None,
            metadatas,
        );
        for neighbor_id in neighbors {
            if neighbor_id == *id {
                continue;
            }
            let (a, b) = if id < &neighbor_id { (*id, neighbor_id) } else { (neighbor_id, *id) };
            if !seen.insert((a, b)) {
                continue;
            }
            if let (Some(va), Some(vb)) = (vectors.get(&a), vectors.get(&b)) {
                let score = metric.calculate(va, vb, mode);
                if score >= threshold {
                    pairs.push(DuplicateHit { id_a: a, id_b: b, score });
                }
            }
        }
    }

    pairs.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    if let Some(max) = limit {
        pairs.truncate(max);
    }
    Ok(pairs)
}
