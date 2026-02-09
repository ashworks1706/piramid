use wide::f32x8;
use crate::config::ExecutionMode;


pub fn euclidean_distance(a: &[f32], b: &[f32], mode: ExecutionMode) -> f32 {
   
    match mode {
        // Explicitly choose the SIMD implementation
        ExecutionMode::Simd => euclidean_distance_simd(a, b),
        
        // Explicitly choose the Scalar implementation
        ExecutionMode::Scalar => euclidean_distance_scalar(a, b),
        
        // Handle Auto: Use runtime detection or a sensible default
        ExecutionMode::Auto => {
            // For example, if using the 'wide' crate, it often manages its own dispatch,
            // but you can manually check for features like AVX2 here.
           
        },

        // Placeholder for other modes
        ExecutionMode::Parallel => {
                    },

        // Exhaustive match ensures all variants are handled
        _ => euclidean_distance_scalar(a, b),
    }

}

fn euclidean_distance_simd(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same length");
    
    let len = a.len();
    let mut sum_sq = f32x8::splat(0.0);
    
    let chunks = len / 8;
    let remainder = len % 8;
    
    for i in 0..chunks {
        let offset = i * 8;
        let va = f32x8::new([
            a[offset], a[offset+1], a[offset+2], a[offset+3],
            a[offset+4], a[offset+5], a[offset+6], a[offset+7],
        ]);
        let vb = f32x8::new([
            b[offset], b[offset+1], b[offset+2], b[offset+3],
            b[offset+4], b[offset+5], b[offset+6], b[offset+7],
        ]);
        
        let diff = va - vb;
        sum_sq += diff * diff;
    }
    
    let mut result: f32 = sum_sq.to_array().iter().sum();
    
    // Handle remainder
    for i in (len - remainder)..len {
        let diff = a[i] - b[i];
        result += diff * diff;
    }
    
    result.sqrt()
}

fn euclidean_distance_scalar(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same length");
    
    let mut sum_sq = 0.0;
    for i in 0..a.len() {
        let diff = a[i] - b[i];
        sum_sq += diff * diff;
    }
    
    sum_sq.sqrt()
}

// Squared distance - skip the sqrt when you only need to compare
// (if a² < b², then a < b, so sqrt is unnecessary for ranking)
pub fn euclidean_distance_squared(a: &[f32], b: &[f32], mode: ExecutionMode) -> f32 {
   
      match mode {
        // Explicitly choose the SIMD implementation
        ExecutionMode::Simd => eeuclidean_distance_squared_simd(a, b),
        
        // Explicitly choose the Scalar implementation
        ExecutionMode::Scalar => euclidean_distance_scalar(a, b),
        
        // Handle Auto: Use runtime detection or a sensible default
        ExecutionMode::Auto => {
            // For example, if using the 'wide' crate, it often manages its own dispatch,
            // but you can manually check for features like AVX2 here.
                   },

        // Placeholder for other modes
        ExecutionMode::Parallel => {
            // e.g., use rayon here
        },

        // Exhaustive match ensures all variants are handled
        _ => euclidean_distance_scalar(a, b),
    }


}

fn euclidean_distance_squared_simd(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same length");
    
    let len = a.len();
    let mut sum_sq = f32x8::splat(0.0);
    
    let chunks = len / 8;
    let remainder = len % 8;
    
    for i in 0..chunks {
        let offset = i * 8;
        let va = f32x8::new([
            a[offset], a[offset+1], a[offset+2], a[offset+3],
            a[offset+4], a[offset+5], a[offset+6], a[offset+7],
        ]);
        let vb = f32x8::new([
            b[offset], b[offset+1], b[offset+2], b[offset+3],
            b[offset+4], b[offset+5], b[offset+6], b[offset+7],
        ]);
        
        let diff = va - vb;
        sum_sq += diff * diff;
    }
    
    let mut result: f32 = sum_sq.to_array().iter().sum();
    
    // Handle remainder
    for i in (len - remainder)..len {
        let diff = a[i] - b[i];
        result += diff * diff;
    }
    
    result
}

fn euclidean_distance_squared_scalar(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same length");
    
    let mut sum_sq = 0.0;
    for i in 0..a.len() {
        let diff = a[i] - b[i];
        sum_sq += diff * diff;
    }
    
    sum_sq
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
