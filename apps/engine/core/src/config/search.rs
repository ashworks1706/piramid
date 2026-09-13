//! Search settings.

use serde::{Deserialize, Serialize};

use piramid_hardware::compute::Metric;

/// The metric a new collection is created with, and how a batch of queries runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct SearchConfig {
    /// Metric a collection is created with. A collection keeps the metric it was created with.
    pub metric: Metric,

    /// Fan a batch of queries across the worker threads.
    pub parallel: bool,
}

impl Default for SearchConfig {
    fn default() -> Self {
        SearchConfig {
            metric: Metric::Cosine,
            parallel: true,
        }
    }
}
