
/// Why cosine? Most LLM embeddings are normalized, meaning the vector's length
/// doesn't matter - only its direction. Cosine ignores magnitude and just
/// measures the angle. 
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    //  assert! panics if condition is false. Good for catching bugs early.
    assert_eq!(a.len(), b.len(), "Vectors must have same length");
    
    let mut dot = 0.0;    // `mut` = mutable, can change
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    // Cosine similarity: cos(θ) = (A · B) / (||A|| × ||B||)
    
    // .iter() gives references, .zip() pairs them up
    // (x, y) destructures the tuple
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }
    
    let denominator = norm_a.sqrt() * norm_b.sqrt();
    
    //  if/else is an expression - returns a value
    if denominator == 0.0 {
        0.0
    } else {
        dot / denominator
    }
}

// #[cfg(test)] = only compile this when running `cargo test`
#[cfg(test)]
mod tests {
    use super::*;  // import everything from parent module

    #[test]  // marks this fn as a test
    fn test_identical_vectors() {
        let v = vec![1.0, 2.0, 3.0];  // vec! macro creates a Vec
        let similarity = cosine_similarity(&v, &v);  // &v = borrow, don't move
        // floats are imprecise, so we check if "close enough"
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
