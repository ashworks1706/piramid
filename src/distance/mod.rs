//! Distance metrics - how we measure "similarity" between vectors

// each file is a module. `mod x` says "include x.rs"
mod cosine;
mod euclidean;
mod dot;

// `pub use` re-exports: users can do `distance::cosine_similarity`
// instead of `distance::cosine::cosine_similarity`
pub use cosine::cosine_similarity;
pub use euclidean::euclidean_distance;
pub use dot::dot_product;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DistanceMetric {
    #[default]  // used when you call DistanceMetric::default()
    Cosine,
    Euclidean,
    DotProduct,
}

impl DistanceMetric {
    pub fn calculate(&self, a: &[f32], b: &[f32]) -> f32 {
        //  `match` must handle ALL variants (exhaustive)
        // This is great - compiler catches if you add a new variant
        match self {
            DistanceMetric::Cosine => cosine_similarity(a, b),
            DistanceMetric::Euclidean => {
                let dist = euclidean_distance(a, b);
                1.0 / (1.0 + dist)  // flip: distance -> similarity
            }
            DistanceMetric::DotProduct => dot_product(a, b),
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
        assert!((DistanceMetric::Cosine.calculate(&v, &v) - 1.0).abs() < 1e-6);
        assert!((DistanceMetric::Euclidean.calculate(&v, &v) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_orthogonal_vectors() {
        let v1 = vec![1.0, 0.0];
        let v2 = vec![0.0, 1.0];
        
        // Orthogonal vectors have 0 cosine similarity
        assert!(DistanceMetric::Cosine.calculate(&v1, &v2).abs() < 1e-6);
    }
}
