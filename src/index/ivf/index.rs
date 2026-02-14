// IVF (Inverted File Index) implementation
// Clusters vectors using k-means, then searches only relevant clusters
// O(√N) search complexity - much faster than brute force for large datasets

use uuid::Uuid;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

use super::config::IvfConfig;
use crate::index::traits::{VectorIndex, IndexStats, IndexDetails, IndexType};

// IVF index structure
#[derive(Clone, Serialize, Deserialize)]
pub struct IvfIndex {
    config: IvfConfig,
    centroids: Vec<Vec<f32>>,                    // Cluster centroids
    inverted_lists: Vec<Vec<Uuid>>,              // vectors[cluster_id] = [vector_ids]
    vector_to_cluster: HashMap<Uuid, usize>,     // Track which cluster each vector belongs to
    dimensions: usize,
}

impl IvfIndex {
    // IVF is different from HNSW
    // It workes by clustering vectors into groups and only searching relevant clusters for a query 
    // This is much faster than brute force for large datasets, but less accurate than HNSW 
    // why? Because it relies on the quality of the clusters - if a query is near a cluster
    // boundary, it may miss relevant vectors in neighboring clusters.
    // IVF is a good choice for very large datasets where search speed is critical and some loss of
    // accuracy is acceptable.
    

    // In contrast, HNSW builds a hierarchical graph structure that allows for more accurate search
    // at the cost of increased memory usage and slower insertion times.
    // IVF is often used in combination with other techniques (like PQ) to further reduce memory
    // usage while maintaining good search performance.
    

    pub fn new(config: IvfConfig) -> Self {
        IvfIndex {
            config,
            centroids: Vec::new(),
            inverted_lists: Vec::new(),
            vector_to_cluster: HashMap::new(),
            dimensions: 0,
        }
    }
    
    // Build clusters using k-means
    pub fn build_clusters(&mut self, vectors: &HashMap<Uuid, Vec<f32>>) {
        // building clusters is an offline process that can be done periodically as new vectors are
        // added
        // on high level, it works by:
        // 1. Randomly initialize centroids from existing vectors
        // 2. Assign each vector to nearest centroid (forming clusters)
        // 3. Update centroids by computing mean of assigned vectors
        // 4. Repeat until convergence or max iterations
        if vectors.is_empty() {
            return;
        }
        
        // Get dimensions from first vector
        if let Some(v) = vectors.values().next() {
            self.dimensions = v.len();
        }
        
        let num_clusters = self.config.num_clusters.min(vectors.len());
        
        // Initialize centroids randomly from existing vectors
        let vector_list: Vec<(Uuid, Vec<f32>)> = vectors.iter()
            .map(|(id, vec)| (*id, vec.clone()))
            .collect();
        
        self.centroids = vector_list.iter()
            .take(num_clusters)
            .map(|(_, v)| v.clone())
            .collect();
        
        // K-means iterations
        for _ in 0..self.config.max_iterations {
            // Assign each vector to nearest centroid
            let mut clusters: Vec<Vec<(Uuid, Vec<f32>)>> = vec![Vec::new(); num_clusters];
            
            for (id, vec) in &vector_list {
                let cluster_id = self.find_nearest_centroid(vec);
                clusters[cluster_id].push((*id, vec.clone()));
            }
            
            // Update centroids
            let mut converged = true;
            for (i, cluster) in clusters.iter().enumerate() {
                if cluster.is_empty() {
                    continue;
                }
                
                let new_centroid = self.compute_centroid(cluster);
                
                // Check convergence
                let distance = self.config.metric.calculate(&self.centroids[i], &new_centroid, self.config.mode);
                if distance < 0.99 {  // If centroids moved significantly
                    converged = false;
                }
                
                self.centroids[i] = new_centroid;
            }
            
            if converged {
                break;
            }
        }
        
        // Build inverted lists
        self.inverted_lists = vec![Vec::new(); num_clusters];
        self.vector_to_cluster.clear();
        
        for (id, vec) in &vector_list {
            let cluster_id = self.find_nearest_centroid(vec);
            self.inverted_lists[cluster_id].push(*id);
            self.vector_to_cluster.insert(*id, cluster_id);
        }
    }
    
    fn find_nearest_centroid(&self, vector: &[f32]) -> usize {
        self.centroids.iter()
            .enumerate()
            .map(|(i, centroid)| {
                let score = self.config.metric.calculate(vector, centroid, self.config.mode);
                (i, score)
            })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0)
    }
    
    fn compute_centroid(&self, cluster: &[(Uuid, Vec<f32>)]) -> Vec<f32> {
        // Compute mean vector for the cluster by summing all vectors and dividing by count 
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
    fn insert(&mut self, id: Uuid, vector: &[f32], vectors: &HashMap<Uuid, Vec<f32>>) {
        // For online insertion, find nearest centroid and add to that cluster
        if self.centroids.is_empty() {
            // First insertion - need to build clusters
            if vectors.len() >= self.config.num_clusters {
                self.build_clusters(vectors);
            }
            return;
        }
        
        let cluster_id = self.find_nearest_centroid(vector);
        
        // Add to inverted list
        if cluster_id < self.inverted_lists.len() {
            if !self.inverted_lists[cluster_id].contains(&id) {
                self.inverted_lists[cluster_id].push(id);
                self.vector_to_cluster.insert(id, cluster_id);
            }
        }
    }
    
    fn search(
        &self,
        query: &[f32],
        k: usize,
        vectors: &HashMap<Uuid, Vec<f32>>,
        quality: crate::config::SearchConfig,
        _filter: Option<&crate::search::query::Filter>,
        _metadatas: &HashMap<Uuid, crate::metadata::Metadata>,
    ) -> Vec<Uuid> {
        if self.centroids.is_empty() {
            // No clusters yet - fallback to brute force
            let mut distances: Vec<(Uuid, f32)> = vectors.iter()
                .map(|(id, vec)| {
                    let score = self.config.metric.calculate(query, vec, self.config.mode);
                    (*id, score)
                })
                .collect();
            
            distances.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            return distances.iter().take(k).map(|(id, _)| *id).collect();
        }
        
        // Find nearest centroids
        let mut centroid_distances: Vec<(usize, f32)> = self.centroids.iter()
            .enumerate()
            .map(|(i, centroid)| {
                let score = self.config.metric.calculate(query, centroid, self.config.mode);
                (i, score)
            })
            .collect();
        
        centroid_distances.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        // Use quality.nprobe if provided, otherwise use configured num_probes
        let nprobe = quality.nprobe.unwrap_or(self.config.num_probes);
        
        // Search top nprobe clusters
        let mut candidates: Vec<(Uuid, f32)> = Vec::new();
        
        for (cluster_id, _) in centroid_distances.iter().take(nprobe) {
            if let Some(vector_ids) = self.inverted_lists.get(*cluster_id) {
                for id in vector_ids {
                    if let Some(vec) = vectors.get(id) {
                        let score = self.config.metric.calculate(query, vec, self.config.mode);
                        candidates.push((*id, score));
                    }
                }
            }
        }
        
        // Sort and return top k
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        candidates.iter().take(k).map(|(id, _)| *id).collect()
    }
    
    fn remove(&mut self, id: &Uuid) {
        if let Some(cluster_id) = self.vector_to_cluster.remove(id) {
            if let Some(list) = self.inverted_lists.get_mut(cluster_id) {
                list.retain(|vid| vid != id);
            }
        }
    }
    
    fn stats(&self) -> IndexStats {
        let vectors_per_cluster = self.inverted_lists.iter()
            .map(|list| list.len())
            .collect();
        
        let memory_usage = 
            self.centroids.len() * self.dimensions * std::mem::size_of::<f32>() +
            self.vector_to_cluster.len() * (std::mem::size_of::<Uuid>() + std::mem::size_of::<usize>()) +
            self.inverted_lists.iter().map(|l| l.len() * std::mem::size_of::<Uuid>()).sum::<usize>();
        
        IndexStats {
            index_type: IndexType::Ivf,
            total_vectors: self.vector_to_cluster.len(),
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ivf_clustering() {
        let mut config = IvfConfig::default();
        config.num_clusters = 2;
        config.num_probes = 2;
        config.max_iterations = 5;
        
        let mut index = IvfIndex::new(config);
        let mut vectors = HashMap::new();
        
        // Create two clusters
        for i in 0..10 {
            let id = Uuid::new_v4();
            let vec = if i < 5 {
                vec![1.0, 0.0, 0.0]  // Cluster 1
            } else {
                vec![0.0, 1.0, 0.0]  // Cluster 2
            };
            vectors.insert(id, vec.clone());
            index.insert(id, &vec, &vectors);
        }
        
        index.build_clusters(&vectors);
        
        assert_eq!(index.centroids.len(), 2);
        assert_eq!(index.inverted_lists.len(), 2);
    }
    
    #[test]
    fn test_ivf_search() {
        let config = IvfConfig::default();
        let mut index = IvfIndex::new(config);
        let mut vectors = HashMap::new();
        
        // Add some vectors
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let v1 = vec![1.0, 0.0, 0.0];
        let v2 = vec![0.9, 0.1, 0.0];
        
        vectors.insert(id1, v1.clone());
        vectors.insert(id2, v2.clone());
        
        index.insert(id1, &v1, &vectors);
        index.insert(id2, &v2, &vectors);
        
        let query = vec![1.0, 0.0, 0.0];
        let empty_meta: HashMap<Uuid, crate::metadata::Metadata> = HashMap::new();
        let results = index.search(&query, 1, &vectors, crate::config::SearchConfig::default(), None, &empty_meta);
        
        assert!(!results.is_empty());
    }
}
