// Scalar implementation of dot product

pub fn dot_product_scalar(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same length");
    
    let mut result = 0.0;
    for i in 0..a.len() {
        result += a[i] * b[i];
    }
    
    result
}
