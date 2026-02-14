// IVF index configuration

use serde::{Serialize, Deserialize};
use crate::metrics::Metric;
use crate::config::ExecutionMode;

// IVF index configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IvfConfig {
    pub num_clusters: usize,      // Number of clusters (√N is a good default)
    pub num_probes: usize,         // Clusters to search (1-10, higher = better recall)
    pub max_iterations: usize,     // K-means iterations
    pub metric: Metric,
    #[serde(default)]
    pub mode: ExecutionMode,
}

// Implement default values for IvfConfig. The default configuration uses 100 clusters, which is a common choice for the number of clusters in IVF. The number of probes is set to 5, which provides a good balance between search quality and speed. The maximum number of iterations for k-means clustering is set to 10, which is typically sufficient for convergence. The default metric is cosine similarity, and the execution mode is set to automatic, allowing the system to choose the best execution strategy based on the environment and workload.
impl Default for IvfConfig {
    fn default() -> Self {
        IvfConfig {
            num_clusters: 100,
            num_probes: 5,
            max_iterations: 10,
            metric: Metric::Cosine,
            mode: ExecutionMode::default(),
        }
    }
}


impl IvfConfig {
    // Auto-configure based on dataset size
    pub fn auto(num_vectors: usize) -> Self {
        let num_clusters = (num_vectors as f32).sqrt().max(10.0) as usize;
        let num_probes = (num_clusters as f32 * 0.1).max(1.0).min(10.0) as usize;
        
        IvfConfig {
            num_clusters,
            num_probes,
            max_iterations: 10,
            metric: Metric::Cosine,
            mode: ExecutionMode::default(),
        }
    }
}
