// This module provides different ways to compare vectors in high-dimensional space.
// Each metric has different characteristics and use cases:
// - Cosine: Best for text embeddings (direction matters, not magnitude)
// - Euclidean: Physical distance in space (magnitude matters)
// - Dot Product: Fast, good for normalized vectors

// each file is a module. `mod x` says "include x.rs"
mod cosine;
mod euclidean;
mod dot;
pub mod latency;

// `pub use` re-exports: users can do `metrics::cosine_similarity`
// instead of `metrics::cosine::cosine_similarity`
pub use cosine::cosine_similarity;
pub use euclidean::euclidean_distance;
pub use dot::dot_product;
pub use latency::{LatencyTracker, time_operation, time_operation_sync};

// Distance/similarity metric for vector comparison.
// 
// Different metrics have different semantics:
// - Similarity metrics (Cosine, DotProduct): higher = more similar
// - Distance metrics (Euclidean): lower = more similar
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Metric {
    #[default]  // used when you call Metric::default()
    Cosine,
    Euclidean,
    DotProduct,
}

impl Metric {
    pub fn calculate(&self, a: &[f32], b: &[f32]) -> f32 {
        //  `match` must handle ALL variants (exhaustive)
        // This is great - compiler catches if you add a new variant
        match self {
            Metric::Cosine => cosine_similarity(a, b),
            Metric::Euclidean => {
                let dist = euclidean_distance(a, b);
                1.0 / (1.0 + dist)  // flip: distance -> similarity
            }
            Metric::DotProduct => dot_product(a, b),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_vectors() {
        let v = vec![1.0, 2.0, 3.0];
        
        // Identical vectors should have max similarity
        assert!((Metric::Cosine.calculate(&v, &v) - 1.0).abs() < 1e-6);
        assert!((Metric::Euclidean.calculate(&v, &v) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_orthogonal_vectors() {
        let v1 = vec![1.0, 0.0];
        let v2 = vec![0.0, 1.0];
        
        // Orthogonal vectors have 0 cosine similarity
        assert!(Metric::Cosine.calculate(&v1, &v2).abs() < 1e-6);
    }
}
