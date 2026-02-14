mod config;
mod graph;

pub use config::{HnswConfig, HnswStats};
pub use graph::HnswIndex;

// Implement VectorIndex trait for HnswIndex
use uuid::Uuid;
use std::collections::HashMap;
use crate::index::traits::{VectorIndex, IndexStats, IndexDetails, IndexType};

impl VectorIndex for HnswIndex {
    fn insert(&mut self, id: Uuid, vector: &[f32], vectors: &HashMap<Uuid, Vec<f32>>) {
        self.insert(id, vector, vectors);
    }
    
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
    
    fn stats(&self) -> IndexStats {
        let hnsw_stats = self.stats();
        
        IndexStats {
            index_type: IndexType::Hnsw,
            total_vectors: hnsw_stats.total_nodes,
            memory_usage_bytes: 0, // TODO: calculate actual memory
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
