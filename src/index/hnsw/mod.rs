mod config;
mod graph;

pub use config::{HnswConfig, HnswStats};
pub use graph::HnswIndex;

use uuid::Uuid;
use std::collections::HashMap;
use crate::index::traits::{VectorIndex, IndexStats, IndexDetails, IndexType};

// Implement the VectorIndex trait for HnswIndex. 
impl VectorIndex for HnswIndex {
    fn insert(&mut self, id: Uuid, vector: &[f32], vectors: &HashMap<Uuid, Vec<f32>>) {
        self.insert(id, vector, vectors);
    }
    
    // Search for nearest neighbors to the query vector with filters.
    fn search(
        &self,
        query: &[f32],
        k: usize,
        vectors: &HashMap<Uuid, Vec<f32>>,
        quality: crate::config::SearchConfig,
        filter: Option<&crate::search::query::Filter>,
        metadatas: &HashMap<Uuid, crate::metadata::Metadata>,
    ) -> Vec<Uuid> {
        // Use quality.ef if provided, otherwise use configured ef_search
        let ef = quality.ef.unwrap_or_else(|| self.get_ef_search()).max(k);
        self.search(query, k, ef, vectors, filter, metadatas)
    }
    
    fn remove(&mut self, id: &Uuid) {
        self.remove(id);
    }
    
    // Get statistics about the HNSW index, including total nodes, max layer, layer sizes, average connections, and memory usage.
    fn stats(&self) -> IndexStats {
        let hnsw_stats = self.stats();
        
        IndexStats {
            index_type: IndexType::Hnsw,
            total_vectors: hnsw_stats.total_nodes,
            memory_usage_bytes: hnsw_stats.memory_usage_bytes,
            details: IndexDetails::Hnsw {
                max_layer: hnsw_stats.max_layer,
                layer_sizes: hnsw_stats.layer_sizes,
                avg_connections: hnsw_stats.avg_connections,
            },
        }
    }
    
    fn index_type(&self) -> IndexType {
        IndexType::Hnsw
    }
}
