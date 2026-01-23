/// Cosine similarity between two vectors
/// 
/// Measures the cosine of the angle between vectors, ignoring magnitude.
/// Ideal for normalized embeddings where direction matters more than length.
/// 
/// Returns value in range [-1, 1] where:
/// - 1.0 = identical direction
/// - 0.0 = orthogonal (perpendicular)
/// - -1.0 = opposite direction
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same length");
    
    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }
    
    let denominator = norm_a.sqrt() * norm_b.sqrt();
    
    if denominator == 0.0 {
        0.0
    } else {
        dot / denominator
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_vectors() {
        let v = vec![1.0, 2.0, 3.0];
        let similarity = cosine_similarity(&v, &v);
        assert!((similarity - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_opposite_vectors() {
        let v1 = vec![1.0, 2.0, 3.0];
        let v2 = vec![-1.0, -2.0, -3.0];
        let similarity = cosine_similarity(&v1, &v2);
        assert!((similarity - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn test_orthogonal_vectors() {
        let v1 = vec![1.0, 0.0];
        let v2 = vec![0.0, 1.0];
        let similarity = cosine_similarity(&v1, &v2);
        assert!(similarity.abs() < 1e-6);
    }

    #[test]
    fn test_zero_vector() {
        let v1 = vec![0.0, 0.0, 0.0];
        let v2 = vec![1.0, 2.0, 3.0];
        let similarity = cosine_similarity(&v1, &v2);
        assert_eq!(similarity, 0.0);
    }

    #[test]
    #[should_panic(expected = "Vectors must have same length")]
    fn test_different_lengths() {
        let v1 = vec![1.0, 2.0];
        let v2 = vec![1.0, 2.0, 3.0];
        cosine_similarity(&v1, &v2);
    }
}
