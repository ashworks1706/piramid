mod config;
mod graph;

pub use config::{HnswConfig, HnswStats};
pub use graph::HnswIndex;

// Implement VectorIndex trait for HnswIndex
use uuid::Uuid;
use std::collections::HashMap;
use crate::index::traits::{VectorIndex, IndexStats, IndexDetails, IndexType};

// Implement the VectorIndex trait for HnswIndex. This includes methods for inserting vectors, searching for nearest neighbors, removing vectors, and getting index statistics. The insert method adds a vector to the HNSW graph structure. The search method performs an approximate nearest neighbor search using the HNSW algorithm, which is more efficient than a brute force search while still providing good accuracy. The remove method removes a vector from the graph, and the stats method returns information about the index such as total nodes, max layer, layer sizes, average connections, and memory usage.
impl VectorIndex for HnswIndex {
    fn insert(&mut self, id: Uuid, vector: &[f32], vectors: &HashMap<Uuid, Vec<f32>>) {
        self.insert(id, vector, vectors);
    }
    
    // Search for nearest neighbors to the query vector. This method uses the HNSW search algorithm, which involves traversing the graph structure to find the closest nodes to the query vector. The quality parameter can be used to adjust the ef parameter of the search, which controls the tradeoff between search speed and accuracy. The filter and metadata parameters can be used to filter results based on metadata or other criteria, although they are not implemented in this basic version.
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
    
    // Get statistics about the HNSW index, including total nodes, max layer, layer sizes, average connections, and memory usage. This information can be useful for monitoring the health of the index and understanding its structure and performance characteristics.
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
