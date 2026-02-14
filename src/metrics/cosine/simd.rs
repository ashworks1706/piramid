// SIMD implementation of cosine similarity
// Uses wide crate for AVX2/NEON vectorization

use wide::f32x8;

pub fn cosine_similarity_simd(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same length");
    
    let len = a.len();
    let mut dot_sum = f32x8::splat(0.0);
    let mut norm_a_sum = f32x8::splat(0.0);
    let mut norm_b_sum = f32x8::splat(0.0);
    
    let chunks = len / 8;
    let remainder = len % 8;
    
    // Process 8 elements at a time using SIMD
    // it works by loading 8 consecutive elements from each vector into SIMD registers (f32x8), then performing element-wise multiplication to compute the dot product and norms in parallel. The results are accumulated in SIMD registers, which are then reduced to scalar values for the final cosine similarity calculation. Any remaining elements that don't fit into a full SIMD register are processed using a simple scalar loop to ensure all elements are included in the computation.
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
