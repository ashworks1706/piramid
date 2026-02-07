// Flat index configuration

use serde::{Serialize, Deserialize};
use crate::metrics::Metric;

// Flat index configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlatConfig {
    pub metric: Metric,
}

impl Default for FlatConfig {
    fn default() -> Self {
        FlatConfig {
            metric: Metric::Cosine,
        }
    }
}
