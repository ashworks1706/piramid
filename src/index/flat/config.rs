// Flat index configuration
// This struct defines the configuration options for a flat index, that stores all vectors in a single list and performs linear search.
use serde::{Serialize, Deserialize};
use crate::metrics::Metric;
use crate::config::ExecutionMode;

// Flat index configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlatConfig {
    pub metric: Metric, // Distance metric to use for similarity calculations (e.g., cosine, euclidean)
    #[serde(default)] 
    pub mode: ExecutionMode, // Execution mode for search operations (e.g., auto, single-threaded, multi-threaded)
}
impl Default for FlatConfig {
    fn default() -> Self {
        FlatConfig {
            metric: Metric::Cosine,
            mode: ExecutionMode::default(),
        }
    }
}
