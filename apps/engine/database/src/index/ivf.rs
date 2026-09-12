//! Inverted-file index: partitions vectors into k-means clusters and searches the nearest ones.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::index::{IndexDetails, IndexStats, IndexType, VectorIndex, VectorReader};
use piramid_core::config::IvfConfig;
use piramid_core::error::{IndexError, Result};
use piramid_hardware::compute::{strategies::for_mode, DistanceKernels};

/// Centroids at least this similar between iterations count as settled.
const CONVERGENCE_SIMILARITY: f32 = 0.99;

/// Vectors partitioned by nearest centroid, searched by probing the closest partitions.
#[derive(Clone, Serialize, Deserialize)]
pub struct IvfIndex {
    config: IvfConfig,
    centroids: Vec<Vec<f32>>,
    inverted_lists: Vec<Vec<Uuid>>, // vectors[cluster_id] = [vector_ids]
    vector_to_cluster: HashMap<Uuid, usize>,
    #[serde(default)]
    pending_vectors: HashSet<Uuid>, // vectors not yet assigned, before the first clustering run
    dimensions: usize,
}

impl IvfIndex {
    /// An empty index with no trained centroids.
    pub fn new(config: IvfConfig) -> Self {
        IvfIndex {
            config,
            centroids: Vec::new(),
            inverted_lists: Vec::new(),
            vector_to_cluster: HashMap::new(),
            pending_vectors: HashSet::new(),
            dimensions: 0,
        }
    }

    /// Train centroids over the vectors with the Lloyd algorithm. Rebuilt periodically, not per
    /// insert.
    pub fn build_clusters(&mut self, vectors: &dyn VectorReader) -> Result<()> {
        if vectors.is_empty() {
            return Ok(());
        }
        let kernels = for_mode(self.config.mode)?;

        let vector_list: Vec<(Uuid, Vec<f32>)> = vectors
            .iter()
            .map(|(id, vector)| (id, vector.to_vec()))
            .collect();
        if let Some((_, vector)) = vector_list.first() {
            self.dimensions = vector.len();
        }

        let num_clusters = self.config.num_clusters.min(vector_list.len());

        self.centroids = vector_list
            .iter()
            .take(num_clusters)
            .map(|(_, v)| v.clone())
            .collect();

        for _ in 0..self.config.max_iterations {
            let mut clusters: Vec<Vec<(Uuid, Vec<f32>)>> = vec![Vec::new(); num_clusters];

            for (id, vec) in &vector_list {
                let cluster_id = self.find_nearest_centroid(vec, kernels)?;
                clusters[cluster_id].push((*id, vec.clone()));
            }

            let mut converged = true;
            for (i, cluster) in clusters.iter().enumerate() {
                if cluster.is_empty() {
                    continue;
                }

                let new_centroid = self.compute_centroid(cluster);

                // Metric::calculate normalises to higher-is-closer, so this is a similarity
                // rather than a distance. The threshold is meaningful only for a bounded metric.
                let similarity =
                    self.config
                        .metric
                        .calculate(&self.centroids[i], &new_centroid, kernels);
                if similarity < CONVERGENCE_SIMILARITY {
                    converged = false;
                }

                self.centroids[i] = new_centroid;
            }

            if converged {
                break;
            }
        }

        self.inverted_lists = vec![Vec::new(); num_clusters];
        self.vector_to_cluster.clear();
        self.pending_vectors.clear();

        for (id, vec) in &vector_list {
            let cluster_id = self.find_nearest_centroid(vec, kernels)?;
            self.inverted_lists[cluster_id].push(*id);
            self.vector_to_cluster.insert(*id, cluster_id);
        }
        Ok(())
    }

    /// Index of the centroid nearest the vector. Errors when there are no centroids yet.
    fn find_nearest_centroid(
        &self,
        vector: &[f32],
        kernels: &dyn DistanceKernels,
    ) -> Result<usize> {
        self.centroids
            .iter()
            .enumerate()
            .map(|(i, centroid)| (i, self.config.metric.calculate(vector, centroid, kernels)))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .ok_or_else(|| IndexError::NotInitialized.into())
    }

    fn compute_centroid(&self, cluster: &[(Uuid, Vec<f32>)]) -> Vec<f32> {
        if cluster.is_empty() {
            return vec![0.0; self.dimensions];
        }

        let mut centroid = vec![0.0; self.dimensions];

        for (_, vec) in cluster {
            for (i, &val) in vec.iter().enumerate() {
                centroid[i] += val;
            }
        }

        let count = cluster.len() as f32;
        for val in &mut centroid {
            *val /= count;
        }

        centroid
    }
}

impl VectorIndex for IvfIndex {
    fn insert(&mut self, id: Uuid, vector: &[f32], vectors: &dyn VectorReader) -> Result<()> {
        if self.vector_to_cluster.contains_key(&id) || self.pending_vectors.contains(&id) {
            return Ok(());
        }

        // Online insert assigns to the nearest existing centroid rather than retraining.
        if self.centroids.is_empty() {
            self.pending_vectors.insert(id);

            if vectors.len() >= self.config.num_clusters {
                self.build_clusters(vectors)?;
            }
            return Ok(());
        }

        let kernels = for_mode(self.config.mode)?;
        let cluster_id = self.find_nearest_centroid(vector, kernels)?;

        let list = self.inverted_lists.get_mut(cluster_id).ok_or_else(|| {
            IndexError::SearchFailed(format!(
                "IVF centroid {cluster_id} has no inverted list; the index needs a rebuild"
            ))
        })?;
        list.push(id);
        self.vector_to_cluster.insert(id, cluster_id);
        Ok(())
    }

    fn search(&self, request: crate::index::IndexSearchRequest<'_>) -> Result<Vec<Uuid>> {
        let crate::index::IndexSearchRequest {
            query,
            k,
            vectors,
            config: quality,
            ..
        } = request;
        if self.centroids.is_empty() {
            return Err(IndexError::NotInitialized.into());
        }
        let kernels = for_mode(self.config.mode)?;

        let mut centroid_distances: Vec<(usize, f32)> = self
            .centroids
            .iter()
            .enumerate()
            .map(|(i, centroid)| (i, self.config.metric.calculate(query, centroid, kernels)))
            .collect();

        centroid_distances
            .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let nprobe = quality.nprobe.unwrap_or(self.config.num_probes);

        // Only the nprobe nearest partitions are scanned.
        let mut candidates: Vec<(Uuid, f32)> = Vec::new();

        for (cluster_id, _) in centroid_distances.iter().take(nprobe) {
            if let Some(vector_ids) = self.inverted_lists.get(*cluster_id) {
                for id in vector_ids {
                    let vector = vectors.get(id).ok_or_else(|| {
                        IndexError::SearchFailed(format!(
                            "IVF index references missing vector {id}"
                        ))
                    })?;
                    let score = self.config.metric.calculate(query, vector, kernels);
                    candidates.push((*id, score));
                }
            }
        }

        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(candidates.iter().take(k).map(|(id, _)| *id).collect())
    }

    fn remove(&mut self, id: &Uuid) {
        self.pending_vectors.remove(id);
        if let Some(cluster_id) = self.vector_to_cluster.remove(id) {
            if let Some(list) = self.inverted_lists.get_mut(cluster_id) {
                list.retain(|vid| vid != id);
            }
        }
    }

    fn stats(&self) -> IndexStats {
        let vectors_per_cluster = self.inverted_lists.iter().map(std::vec::Vec::len).collect();

        let memory_usage = self.centroids.len() * self.dimensions * std::mem::size_of::<f32>()
            + self.vector_to_cluster.len()
                * (std::mem::size_of::<Uuid>() + std::mem::size_of::<usize>())
            + self.pending_vectors.len() * std::mem::size_of::<Uuid>()
            + self
                .inverted_lists
                .iter()
                .map(|l| l.len() * std::mem::size_of::<Uuid>())
                .sum::<usize>();

        IndexStats {
            index_type: IndexType::Ivf,
            total_vectors: self.vector_to_cluster.len() + self.pending_vectors.len(),
            memory_usage_bytes: memory_usage,
            details: IndexDetails::Ivf {
                num_clusters: self.centroids.len(),
                vectors_per_cluster,
                centroids_computed: !self.centroids.is_empty(),
            },
        }
    }

    fn index_type(&self) -> IndexType {
        IndexType::Ivf
    }

    fn to_serializable(&self) -> crate::index::SerializableIndex {
        crate::index::SerializableIndex::Ivf(self.clone())
    }
}
