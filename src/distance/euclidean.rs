/// The straight-line distance works well when
/// vector magnitude carries meaning (e.g., importance scores).
/// Less common for text embeddings, but useful for image/audio.
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same length");
    // Euclidean (L2) distance: d = √(Σ(aᵢ - bᵢ)²)
    let sum_sq: f32 = a.iter()
        .zip(b.iter())
        .map(|(x, y)| {
            let diff = x - y;
            diff * diff
        })
        .sum();
    
    sum_sq.sqrt()
}

/// Squared distance - skip the sqrt when you only need to compare
/// (if a² < b², then a < b, so sqrt is unnecessary for ranking)
pub fn euclidean_distance_squared(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same length");
    
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| {
            let diff = x - y;
            diff * diff
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_vectors() {
        let v = vec![1.0, 2.0, 3.0];
        assert_eq!(euclidean_distance(&v, &v), 0.0);
    }

    #[test]
    fn test_3_4_5_triangle() {
        let v1 = vec![0.0, 0.0];
        let v2 = vec![3.0, 4.0];
        let distance = euclidean_distance(&v1, &v2);
        assert!((distance - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_unit_distance() {
        let v1 = vec![0.0];
        let v2 = vec![1.0];
        assert!((euclidean_distance(&v1, &v2) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_squared_distance() {
        let v1 = vec![0.0, 0.0];
        let v2 = vec![3.0, 4.0];
        assert!((euclidean_distance_squared(&v1, &v2) - 25.0).abs() < 1e-6);
    }

    #[test]
    #[should_panic(expected = "Vectors must have same length")]
    fn test_different_lengths() {
        let v1 = vec![1.0, 2.0];
        let v2 = vec![1.0, 2.0, 3.0];
        euclidean_distance(&v1, &v2);
    }
}
