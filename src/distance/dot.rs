/// The simplest similarity measure - just multiply and sum.
/// Fun fact: for normalized vectors (length = 1), dot product = cosine similarity.
/// Many production systems use dot product because it's faster and embeddings
/// are often pre-normalized anyway.
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same length");
    // Dot product: A · B = Σ(aᵢ × bᵢ)

    a.iter()
        .zip(b.iter())
        .map(|(x, y)| x * y)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_dot_product() {
        let v1 = vec![1.0, 2.0, 3.0];
        let v2 = vec![4.0, 5.0, 6.0];
        let result = dot_product(&v1, &v2);
        // 1*4 + 2*5 + 3*6 = 4 + 10 + 18 = 32
        assert!((result - 32.0).abs() < 1e-6);
    }

    #[test]
    fn test_orthogonal_vectors() {
        let v1 = vec![1.0, 0.0];
        let v2 = vec![0.0, 1.0];
        assert!(dot_product(&v1, &v2).abs() < 1e-6);
    }

    #[test]
    fn test_unit_vectors() {
        let v1 = vec![1.0, 0.0, 0.0];
        let v2 = vec![1.0, 0.0, 0.0];
        assert!((dot_product(&v1, &v2) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_negative_values() {
        let v1 = vec![1.0, -1.0];
        let v2 = vec![-1.0, 1.0];
        assert!((dot_product(&v1, &v2) - (-2.0)).abs() < 1e-6);
    }

    #[test]
    #[should_panic(expected = "Vectors must have same length")]
    fn test_different_lengths() {
        let v1 = vec![1.0, 2.0];
        let v2 = vec![1.0, 2.0, 3.0];
        dot_product(&v1, &v2);
    }
}
