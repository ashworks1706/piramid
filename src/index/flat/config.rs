// Flat index configuration

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

impl Default for FlatConfig {
    fn default() -> Self {
        FlatConfig {
            metric: Metric::Cosine,
            mode: ExecutionMode::default(),
        }
    }
}
