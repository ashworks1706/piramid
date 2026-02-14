// HNSW types and configuration

use serde::{Serialize, Deserialize};
use crate::metrics::Metric;
use crate::config::ExecutionMode;

// HNSW (Hierarchical Navigable Small World) index configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnswConfig{
    pub m: usize,  // max number of connections per node
    pub m_max: usize,  // max connections for layer 0 (typically 2*M)
    pub ef_construction: usize,  // size of the dynamic list for the construction phase
    pub ef_search: usize,  // size of the dynamic list for the search phase (quality vs speed tradeoff)
    pub ml: f32,  // layer multiplier: 1/ln(M)
    pub metric: Metric,  // similarity metric
    #[serde(default)]
    pub mode: ExecutionMode,  // SIMD execution mode
}

// Implement default values for HnswConfig. The default configuration uses M=16, which is a common choice for the number of connections per node in HNSW. The ef_construction and ef_search parameters are both set to 200, which provides a good balance between search quality and speed. The layer multiplier is calculated as 1/ln(M), which is a common setting for HNSW. The default metric is cosine similarity, and the execution mode is set to automatic, allowing the system to choose the best execution strategy based on the environment and workload.
impl Default for HnswConfig {
    fn default() -> Self {
        let m = 16;
        HnswConfig {
            m,
            m_max: m * 2,
            ef_construction: 200,
            ef_search: 200,  // Default to same as ef_construction
            ml: 1.0 / (m as f32).ln(),
            metric: Metric::Cosine,
            mode: ExecutionMode::default(),
        }
    }
}

#[derive(Debug)]
pub struct HnswStats {
    pub total_nodes: usize,
    pub max_layer: isize,
    pub layer_sizes: Vec<usize>,
    pub tombstones: usize,
    pub avg_connections: f32,
    pub memory_usage_bytes: usize,
}
