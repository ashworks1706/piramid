// Flat (brute force) index implementation
// O(N) search - compares query against all vectors
// Best for: small collections (<10k vectors), zero build time, 100% recall

use uuid::Uuid;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

use super::config::FlatConfig;
use crate::index::traits::{VectorIndex, IndexStats, IndexDetails, IndexType};

// Flat index - simple brute force search
// Stores nothing except config (vectors are in main storage)
#[derive(Clone, Serialize, Deserialize)]
pub struct FlatIndex {
    config: FlatConfig,
    vector_ids: Vec<Uuid>,  // Track which vectors we've seen
}

impl FlatIndex {
    pub fn new(config: FlatConfig) -> Self {
        FlatIndex {
            config,
            vector_ids: Vec::new(),
        }
    }
}

impl VectorIndex for FlatIndex {
    fn insert(&mut self, id: Uuid, _vector: &[f32], _vectors: &HashMap<Uuid, Vec<f32>>) {
        // Just track the ID - no indexing structure needed
        if !self.vector_ids.contains(&id) {
            self.vector_ids.push(id);
        }
    }
    
    fn search(
        &self,
        query: &[f32],
        k: usize,
        vectors: &HashMap<Uuid, Vec<f32>>,
        _quality: crate::config::SearchConfig,
        _filter: Option<&crate::search::query::Filter>,
        _metadatas: &HashMap<Uuid, crate::metadata::Metadata>,
    ) -> Vec<Uuid> {
        // Flat index is always exhaustive - quality parameter is ignored
        // Brute force: calculate distance to every vector
        let mut distances: Vec<(Uuid, f32)> = self.vector_ids
            .iter()
            .filter_map(|id| {
                vectors.get(id).map(|vec| {
                    let score = self.config.metric.calculate(query, vec, self.config.mode);
                    (*id, score)
                })
            })
            .collect();
        
        // Sort by score (descending for similarity)
        distances.sort_by(|a, b| {
            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // Return top k IDs
        distances.iter()
            .take(k)
            .map(|(id, _)| *id)
            .collect()
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flat_insert_and_search() {
        let mut index = FlatIndex::new(FlatConfig::default());
        let mut vectors = HashMap::new();
        
        // Insert some vectors
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let id3 = Uuid::new_v4();
        
        let v1 = vec![1.0, 0.0, 0.0];
        let v2 = vec![0.0, 1.0, 0.0];
        let v3 = vec![0.9, 0.1, 0.0];
        
        vectors.insert(id1, v1.clone());
        vectors.insert(id2, v2.clone());
        vectors.insert(id3, v3.clone());
        
        index.insert(id1, &v1, &vectors);
        index.insert(id2, &v2, &vectors);
        index.insert(id3, &v3, &vectors);
        
        // Search for nearest to [1, 0, 0]
        let query = vec![1.0, 0.0, 0.0];
        let results = index.search(&query, 2, &vectors, crate::config::SearchConfig::default());
        
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], id1);
    }
    
    #[test]
    fn test_flat_remove() {
        let mut index = FlatIndex::new(FlatConfig::default());
        let mut vectors = HashMap::new();
        
        let id1 = Uuid::new_v4();
        let v1 = vec![1.0, 0.0, 0.0];
        vectors.insert(id1, v1.clone());
        
        index.insert(id1, &v1, &vectors);
        assert_eq!(index.stats().total_vectors, 1);
        
        index.remove(&id1);
        assert_eq!(index.stats().total_vectors, 0);
    }
}
