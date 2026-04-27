

pub mod cosine;
pub mod euclidean;
pub mod dot;
pub mod latency;
pub mod embed;

pub use cosine::cosine_similarity;
pub use euclidean::{euclidean_distance, euclidean_distance_squared};
pub use dot::dot_product;
pub use latency::{LatencyTracker, time_operation, time_operation_sync};
pub use embed::{EmbedMetrics, EmbedMetricsSnapshot};

use crate::config::ExecutionMode;

// Similarity metrics (Cosine, DotProduct): higher = more similar
// Distance metrics (Euclidean): lower = more similar
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Metric {
    #[default]
    Cosine,
    Euclidean,
    DotProduct,
}

impl Metric {
    pub fn calculate(&self, a: &[f32], b: &[f32], mode: ExecutionMode) -> f32 {
        match self {
            Metric::Cosine => cosine_similarity(a, b, mode),
            Metric::Euclidean => {
                let dist = euclidean_distance(a, b, mode);
                1.0 / (1.0 + dist)
            }
            Metric::DotProduct => dot_product(a, b, mode),
        }
    }
}
