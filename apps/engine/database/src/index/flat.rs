//! Brute-force index: compares the query against every vector, O(N), with perfect recall.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::index::{IndexDetails, IndexStats, IndexType, VectorIndex, VectorReader};
use piramid_core::config::FlatConfig;
use piramid_core::error::{IndexError, Result};
use piramid_hardware::compute::strategies::for_mode;
use piramid_hardware::compute::{DistanceKernels, Metric};

/// Rows scored per batch call when the vectors have to be gathered.
///
/// The gather buffer holds CHUNK * dim floats and lives for one query.
const CHUNK: usize = 1024;

/// An index that scores the query against every vector it holds.
#[derive(Clone, Serialize, Deserialize)]
pub struct FlatIndex {
    config: FlatConfig,
    vector_ids: Vec<Uuid>,
}

impl FlatIndex {
    /// An empty index with the given metric and execution mode.
    pub fn new(config: FlatConfig) -> Self {
        FlatIndex {
            config,
            vector_ids: Vec::new(),
        }
    }

    /// Score every vector this index owns, one batch call per block.
    fn score_all(
        &self,
        query: &[f32],
        vectors: &dyn VectorReader,
        kernels: &dyn DistanceKernels,
    ) -> Result<Vec<(Uuid, f32)>> {
        if self.vector_ids.is_empty() {
            return Ok(Vec::new());
        }
        // A slab covering every indexed row goes to the kernel in one call.
        if let Some(slab) = vectors.as_slab() {
            if slab.rows() == self.vector_ids.len() {
                debug_assert!(
                    slab.ids.iter().all(|id| self.vector_ids.contains(id)),
                    "the flat index and the vector store disagree about what is stored"
                );
                return self.score_slab(query, slab.data, slab.dim, slab.ids, kernels);
            }
        }
        self.score_gathered(query, vectors, kernels)
    }

    fn score_slab(
        &self,
        query: &[f32],
        data: &[f32],
        dim: usize,
        ids: &[Uuid],
        kernels: &dyn DistanceKernels,
    ) -> Result<Vec<(Uuid, f32)>> {
        let mut out = vec![0.0; ids.len()];
        self.metric()
            .calculate_batch(query, data, dim, &mut out, kernels)
            .map_err(|e| IndexError::SearchFailed(e.to_string()))?;
        Ok(ids.iter().copied().zip(out).collect())
    }

    /// Copy a block of rows into a scratch buffer, score it, and move to the next block.
    fn score_gathered(
        &self,
        query: &[f32],
        vectors: &dyn VectorReader,
        kernels: &dyn DistanceKernels,
    ) -> Result<Vec<(Uuid, f32)>> {
        let dim = vectors.dim().ok_or_else(|| {
            IndexError::SearchFailed("Flat index cannot score an empty vector store".to_string())
        })?;
        let mut scored = Vec::with_capacity(self.vector_ids.len());
        let mut block = vec![0.0; CHUNK * dim];
        let mut out = vec![0.0; CHUNK];
        for ids in self.vector_ids.chunks(CHUNK) {
            let rows = ids.len();
            let block = &mut block[..rows * dim];
            let out = &mut out[..rows];
            vectors.gather_into(ids, block).ok_or_else(|| {
                IndexError::SearchFailed(
                    "Flat index references a vector the store does not hold".to_string(),
                )
            })?;
            self.metric()
                .calculate_batch(query, block, dim, out, kernels)
                .map_err(|e| IndexError::SearchFailed(e.to_string()))?;
            scored.extend(ids.iter().copied().zip(out.iter().copied()));
        }
        Ok(scored)
    }

    fn metric(&self) -> Metric {
        self.config.metric
    }
}
impl VectorIndex for FlatIndex {
    fn insert(&mut self, id: Uuid, _vector: &[f32], _vectors: &dyn VectorReader) -> Result<()> {
        // Only the id list is maintained.
        if !self.vector_ids.contains(&id) {
            self.vector_ids.push(id);
        }
        Ok(())
    }

    // Scans every vector. Filters are ignored here and applied by the caller after ranking.
    fn search(&self, request: crate::index::IndexSearchRequest<'_>) -> Result<Vec<Uuid>> {
        let crate::index::IndexSearchRequest {
            query, k, vectors, ..
        } = request;
        let kernels = for_mode(self.config.mode)?;
        let mut scored = self.score_all(query, vectors, kernels)?;

        scored.retain(|(_, score)| !score.is_nan());
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        Ok(scored.into_iter().take(k).map(|(id, _)| id).collect())
    }

    fn remove(&mut self, id: &Uuid) {
        self.vector_ids.retain(|vid| vid != id);
    }

    fn stats(&self) -> IndexStats {
        IndexStats {
            index_type: IndexType::Flat,
            total_vectors: self.vector_ids.len(),
            memory_usage_bytes: self.vector_ids.len() * std::mem::size_of::<Uuid>(),
            details: IndexDetails::Flat,
        }
    }

    fn index_type(&self) -> IndexType {
        IndexType::Flat
    }

    fn metric(&self) -> piramid_hardware::compute::Metric {
        self.config.metric
    }

    fn set_execution(&mut self, mode: piramid_hardware::compute::ExecutionMode) {
        self.config.mode = mode;
    }

    fn build_config(&self) -> piramid_core::config::IndexConfig {
        piramid_core::config::IndexConfig::Flat {
            params: piramid_core::config::FlatConfig {
                mode: piramid_hardware::compute::ExecutionMode::default(),
                ..self.config
            },
        }
    }

    fn to_serializable(&self) -> crate::index::SerializableIndex {
        crate::index::SerializableIndex::Flat(self.clone())
    }
}
