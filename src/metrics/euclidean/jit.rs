// JIT-optimized Euclidean distance
// this works but is not as fast as the dot product version, likely due to the extra sqrt and more
// complex math. Still faster than the non-jit version though.
// it is basically the same as the dot product jit but with the subtraction and squaring instead of
// multiplication when we pass into this function a and b where a is the query vector and b is the
// database vector, then we are computing the distance between them, which is sqrt(sum((a[i] -
// b[i])^2)) by first computing the sum of squares of differences and then taking the square root
// at the end. The reason we have the separate function for squared distance is that sometimes we
// only care about the relative distances and can skip the sqrt for better performance, and also to
// avoid the overhead of the sqrt when we are doing a lot of distance comparisons and only care
// about which one is smaller, not the actual distance value.

pub fn euclidean_distance_jit(a: &[f32], b: &[f32]) -> f32 {
    euclidean_distance_squared_jit(a, b).sqrt()
}

pub fn euclidean_distance_squared_jit(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same length");

    let len = a.len();

    match len {
        1536 => euclidean_squared_jit_1536(a, b),
        _ => euclidean_squared_jit_generic(a, b),
    }
}

#[inline(always)]
fn euclidean_squared_jit_1536(a: &[f32], b: &[f32]) -> f32 {
    let mut sum_sq = 0.0;

    let mut i = 0;
    while i < 1536 {
        let d0 = a[i] - b[i];
        let d1 = a[i + 1] - b[i + 1];
        let d2 = a[i + 2] - b[i + 2];
        let d3 = a[i + 3] - b[i + 3];
        let d4 = a[i + 4] - b[i + 4];
        let d5 = a[i + 5] - b[i + 5];
        let d6 = a[i + 6] - b[i + 6];
        let d7 = a[i + 7] - b[i + 7];

        sum_sq += d0 * d0 + d1 * d1 + d2 * d2 + d3 * d3 + d4 * d4 + d5 * d5 + d6 * d6 + d7 * d7;

        i += 8;
    }

    sum_sq
}

fn euclidean_squared_jit_generic(a: &[f32], b: &[f32]) -> f32 {
    let mut sum_sq = 0.0;
    for i in 0..a.len() {
        let diff = a[i] - b[i];
        sum_sq += diff * diff;
    }
    sum_sq
}
