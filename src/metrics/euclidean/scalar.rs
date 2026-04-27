// Scalar implementation of Euclidean distance

pub fn euclidean_distance_scalar(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same length");
    
    let mut sum_sq = 0.0;
    for i in 0..a.len() {
        let diff = a[i] - b[i];
        sum_sq += diff * diff;
    }
    
    sum_sq.sqrt()
}

pub fn euclidean_distance_squared_scalar(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same length");
    
    let mut sum_sq = 0.0;
    for i in 0..a.len() {
        let diff = a[i] - b[i];
        sum_sq += diff * diff;
    }
    
    sum_sq
}
