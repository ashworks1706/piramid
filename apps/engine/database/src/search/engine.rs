//! Query execution: scores the query against every stored vector and keeps the best k that match.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use crate::storage::vectors::VectorReader;
use piramid_core::error::{Result, StorageError};
use piramid_core::metadata::{Filter, Metadata};
use piramid_core::Document;
use piramid_core::Hit;
use piramid_hardware::compute::{strategies::for_mode, DistanceKernels, ExecutionMode, Metric};
use uuid::Uuid;

/// Rows scored per batch call when the vectors have to be gathered.
const CHUNK: usize = 1024;

/// Per-query options.
#[derive(Debug, Clone, Copy)]
pub struct SearchParams<'a> {
    /// Strategy to score with. Auto resolves as [for_mode] resolves it.
    pub mode: ExecutionMode,
    /// Metadata predicate every hit satisfies.
    pub filter: Option<&'a Filter>,
}

impl Default for SearchParams<'_> {
    fn default() -> Self {
        Self {
            mode: ExecutionMode::Auto,
            filter: None,
        }
    }
}

/// What a search runs against, as borrowed views.
pub struct SearchTarget<'a> {
    /// Every stored vector.
    pub vectors: &'a dyn VectorReader,
    /// The metadata of every stored document, for filter evaluation.
    pub metadata: &'a HashMap<Uuid, Metadata>,
}

/// The k best hits for query under metric among the documents matching the filter of params,
/// best first.
///
/// Every stored vector is scored, and slab rows marked as holes are skipped. A NaN score never ranks. Fewer than k hits come back only when
/// fewer than k documents match. resolve reads the document behind a ranked id.
///
/// # Errors
///
/// Errors when the strategy is unavailable, when the query width differs from the stored width,
/// when scoring fails, or with [StorageError::CorruptedData] when a stored id has no vector or a
/// ranked id does not resolve to a document.
pub fn search(
    target: &SearchTarget<'_>,
    query: &[f32],
    k: usize,
    metric: Metric,
    params: SearchParams<'_>,
    resolve: &(dyn Fn(&Uuid) -> Result<Option<Document>> + Sync),
) -> Result<Vec<Hit>> {
    let kernels = for_mode(params.mode)?;
    let Some(dim) = target.vectors.dim() else {
        return Ok(Vec::new());
    };
    if k == 0 {
        return Ok(Vec::new());
    }
    piramid_core::validation::validate_dimensions(query, dim)?;

    let mut top = TopK::new(k, target.vectors.len(), target.metadata, params.filter);
    match target.vectors.as_slab() {
        Some(slab) => {
            let mut scores = vec![0.0; slab.rows()];
            score(query, slab.data, slab.dim, &mut scores, metric, kernels)?;
            for ((id, live), score) in slab.ids.iter().zip(slab.live).zip(scores) {
                if *live {
                    top.offer(*id, score);
                }
            }
        }
        None => {
            let rows = CHUNK.min(target.vectors.len());
            let mut ids: Vec<Uuid> = Vec::with_capacity(rows);
            let mut block = vec![0.0; rows * dim];
            let mut out = vec![0.0; rows];
            let mut matching = target
                .vectors
                .iter()
                .map(|(id, _)| id)
                .filter(|id| matches(target.metadata, params.filter, id))
                .peekable();
            while matching.peek().is_some() {
                ids.clear();
                ids.extend(matching.by_ref().take(CHUNK));
                let block = &mut block[..ids.len() * dim];
                let out = &mut out[..ids.len()];
                target.vectors.gather_into(&ids, block).ok_or_else(|| {
                    StorageError::CorruptedData("a stored id has no resident vector".to_string())
                })?;
                score(query, block, dim, out, metric, kernels)?;
                for (id, score) in ids.iter().zip(out.iter()) {
                    top.offer_matching(*id, *score);
                }
            }
        }
    }

    top.into_ranked()
        .into_iter()
        .map(|candidate| {
            let document = resolve(&candidate.id)?.ok_or_else(|| {
                StorageError::CorruptedData(format!(
                    "ranked document {} is not in the record store",
                    candidate.id
                ))
            })?;
            Ok(Hit {
                score: candidate.score,
                document,
            })
        })
        .collect()
}

/// Run [search] for each query, in parallel across queries when parallel is set. One hit list
/// per query, in query order.
///
/// # Errors
///
/// Errors when any query errors as [search] does.
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

fn score(
    query: &[f32],
    rows: &[f32],
    dim: usize,
    out: &mut [f32],
    metric: Metric,
    kernels: &dyn DistanceKernels,
) -> Result<()> {
    Ok(metric.calculate_batch(query, rows, dim, out, kernels)?)
}

/// Whether the document id satisfies filter. A document with no metadata entry does not.
fn matches(metadata: &HashMap<Uuid, Metadata>, filter: Option<&Filter>, id: &Uuid) -> bool {
    match filter {
        None => true,
        Some(filter) => metadata
            .get(id)
            .is_some_and(|metadata| filter.matches(metadata)),
    }
}

/// A scored id. Ordered so the greater candidate is the worse one: lower score, then higher id.
#[derive(Clone, Copy)]
struct Candidate {
    score: f32,
    id: Uuid,
}

impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .score
            .total_cmp(&self.score)
            .then_with(|| self.id.cmp(&other.id))
    }
}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Candidate {}

/// The best k matching candidates offered so far, with the worst of them on top of the heap.
struct TopK<'a> {
    k: usize,
    heap: BinaryHeap<Candidate>,
    metadata: &'a HashMap<Uuid, Metadata>,
    filter: Option<&'a Filter>,
}

impl<'a> TopK<'a> {
    /// An empty top k over at most stored candidates.
    fn new(
        k: usize,
        stored: usize,
        metadata: &'a HashMap<Uuid, Metadata>,
        filter: Option<&'a Filter>,
    ) -> Self {
        Self {
            k,
            heap: BinaryHeap::with_capacity(k.min(stored)),
            metadata,
            filter,
        }
    }

    /// Whether a candidate would enter the current top k, before the filter is consulted.
    fn improves(&self, candidate: &Candidate) -> bool {
        if candidate.score.is_nan() {
            return false;
        }
        self.heap.len() < self.k || self.heap.peek().is_some_and(|worst| candidate < worst)
    }

    /// Offer a candidate that has not been checked against the filter.
    fn offer(&mut self, id: Uuid, score: f32) {
        let candidate = Candidate { score, id };
        if self.improves(&candidate) && matches(self.metadata, self.filter, &id) {
            self.push(candidate);
        }
    }

    /// Offer a candidate already known to satisfy the filter.
    fn offer_matching(&mut self, id: Uuid, score: f32) {
        let candidate = Candidate { score, id };
        if self.improves(&candidate) {
            self.push(candidate);
        }
    }

    fn push(&mut self, candidate: Candidate) {
        if self.heap.len() == self.k {
            self.heap.pop();
        }
        self.heap.push(candidate);
    }

    /// The kept candidates, best first.
    fn into_ranked(self) -> Vec<Candidate> {
        self.heap.into_sorted_vec()
    }
}
