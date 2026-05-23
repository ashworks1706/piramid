// SIMD implementation of Euclidean distance
// Uses wide crate for AVX2/NEON vectorization

use wide::f32x8;

pub fn euclidean_distance_simd(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same length");

    let len = a.len();
    let mut sum_sq = f32x8::splat(0.0);

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

pub fn euclidean_distance_squared_simd(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same length");

    let len = a.len();
    let mut sum_sq = f32x8::splat(0.0);

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
