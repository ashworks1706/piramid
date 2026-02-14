// SIMD implementation of dot product
// Uses wide crate for AVX2/NEON vectorization

use wide::f32x8;

pub fn dot_product_simd(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same length");
    let len = a.len();
    let mut sum = f32x8::splat(0.0);
    let chunks = len / 8;
    let remainder = len % 8;

    // Process 8 elements at a time using SIMD. The loop iterates over the input vectors in chunks of 8 (the size of f32x8), loading each chunk into SIMD registers, performing element-wise multiplication, and accumulating the results in the sum register. After processing all full chunks, any remaining elements that don't fit into a full SIMD register are handled in a separate loop to ensure all elements contribute to the final dot product.
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
