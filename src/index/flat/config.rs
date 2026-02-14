// Flat index configuration
// This struct defines the configuration options for a flat index, which is a simple type of vector index that stores all vectors in a single list and performs linear search. The configuration includes the distance metric to use for similarity calculations (e.g., cosine, euclidean) and the execution mode (e.g., auto, single-threaded, multi-threaded). The default configuration uses cosine similarity and automatic execution mode.
use serde::{Serialize, Deserialize};
use crate::metrics::Metric;
use crate::config::ExecutionMode;

// Flat index configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlatConfig {
    pub metric: Metric,
    #[serde(default)]
    pub mode: ExecutionMode,
}
// Implement default values for FlatConfig. By default, we will use cosine similarity as the distance metric and automatic execution mode, which allows the system to choose the best execution strategy based on the environment and workload.
impl Default for FlatConfig {
    fn default() -> Self {
        FlatConfig {
            metric: Metric::Cosine,
            mode: ExecutionMode::default(),
        }
    }
}
