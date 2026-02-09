// Cosine similarity between two vectors
// Measures the cosine of the angle between vectors, ignoring magnitude.
// Ideal for normalized embeddings where direction matters more than length.
// 
// Returns value in range [-1, 1] where:
// - 1.0 = identical direction
// - 0.0 = orthogonal (perpendicular)
// - -1.0 = opposite direction

use wide::f32x8;
use crate::config::ExecutionMode;

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    cosine_similarity_with_mode(a, b, ExecutionMode::default())
}

pub fn cosine_similarity_with_mode(a: &[f32], b: &[f32], mode: ExecutionMode) -> f32 {
    if mode.should_use_simd() {
        cosine_similarity_simd(a, b)
    } else {
        cosine_similarity_scalar(a, b)
    }
}

fn cosine_similarity_simd(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same length");
    
    let len = a.len();
    let mut dot_sum = f32x8::splat(0.0);
    let mut norm_a_sum = f32x8::splat(0.0);
    let mut norm_b_sum = f32x8::splat(0.0);
    
    let chunks = len / 8;
    let remainder = len % 8;
    
    // this for loop processes 8 elements at a time using SIMD vectors to compute the dot product
    // and norms in parallel. It loads 8 elements from each vector into f32x8 SIMD registers,
    // performs the necessary multiplications, and accumulates the results into the dot_sum,
    // norm_a_sum, and norm_b_sum SIMD registers.
    for i in 0..chunks { // where i goes from 0 to number of 8-element chunks
        let offset = i * 8;
        let va = f32x8::new([
            a[offset], a[offset+1], a[offset+2], a[offset+3],
            a[offset+4], a[offset+5], a[offset+6], a[offset+7],
        ]);
        let vb = f32x8::new([
            b[offset], b[offset+1], b[offset+2], b[offset+3],
            b[offset+4], b[offset+5], b[offset+6], b[offset+7],
        ]);
        
        dot_sum += va * vb;
        norm_a_sum += va * va;
        norm_b_sum += vb * vb;
    }
    
    let mut dot: f32 = dot_sum.to_array().iter().sum();
    let mut norm_a: f32 = norm_a_sum.to_array().iter().sum();
    let mut norm_b: f32 = norm_b_sum.to_array().iter().sum();
    
    // Handle remainder
    for i in (len - remainder)..len {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }
    
    let denominator = norm_a.sqrt() * norm_b.sqrt();
    
    if denominator == 0.0 {
        0.0
    } else {
        dot / denominator
    }
}

fn cosine_similarity_scalar(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same length");
    
    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;
    
    for i in 0..a.len() {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
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
