use wide::f32x8;
use crate::config::ExecutionMode;

pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    dot_product_with_mode(a, b, ExecutionMode::default())
}

pub fn dot_product_with_mode(a: &[f32], b: &[f32], mode: ExecutionMode) -> f32 {
    if mode.should_use_simd() {
        dot_product_simd(a, b)
    } else {
        dot_product_scalar(a, b)
    }
}

fn dot_product_simd(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same length");
    let len = a.len();
    let mut sum = f32x8::splat(0.0);
    let chunks = len / 8;
    let remainder = len % 8;

    for i in 0..chunks {
        let offset = i * 8;
        let va = f32x8::new([
            a[offset],
            a[offset + 1],
            a[offset + 2],
            a[offset + 3],
            a[offset + 4],
            a[offset + 5],
            a[offset + 6],
            a[offset + 7],
            ]);
        let vb = f32x8::new([
            b[offset],
            b[offset + 1],
            b[offset + 2],
            b[offset + 3],
            b[offset + 4],
            b[offset + 5],
            b[offset + 6],
            b[offset + 7],
            ]);

        sum += va * vb;
    }

    let mut result: f32 = sum.to_array().iter().sum();
    // Handle remaining elements
    for i in (len - remainder)..len {
        result += a[i] * b[i];
    }

    result
}

fn dot_product_scalar(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same length");
    
    let mut result = 0.0;
    for i in 0..a.len() {
        result += a[i] * b[i];
    }
    
    result
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

    #[test]
    fn test_simd_large_vectors() {
        // Test with realistic embedding size (OpenAI uses 1536)
        let size = 1536;
        let v1: Vec<f32> = (0..size).map(|i| i as f32 * 0.1).collect();
        let v2: Vec<f32> = (0..size).map(|i| (i + 1) as f32 * 0.2).collect();
        
        let result = dot_product(&v1, &v2);
        
        // Verify it's reasonable (should be a large positive number)
        assert!(result > 0.0);
        assert!(result.is_finite());
    }
}
