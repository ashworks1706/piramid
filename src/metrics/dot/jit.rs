// JIT-optimized dot product

pub fn dot_product_jit(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same length");

    let len = a.len();

    match len {
        1536 => dot_product_jit_1536(a, b),
        _ => dot_product_jit_generic(a, b),
    }
}

#[inline(always)]
fn dot_product_jit_1536(a: &[f32], b: &[f32]) -> f32 {
    let mut result = 0.0;

    let mut i = 0;
    while i < 1536 {
        result += a[i] * b[i]
            + a[i + 1] * b[i + 1]
            + a[i + 2] * b[i + 2]
            + a[i + 3] * b[i + 3]
            + a[i + 4] * b[i + 4]
            + a[i + 5] * b[i + 5]
            + a[i + 6] * b[i + 6]
            + a[i + 7] * b[i + 7];

        i += 8;
    }

    result
}

fn dot_product_jit_generic(a: &[f32], b: &[f32]) -> f32 {
    let mut result = 0.0;
    for i in 0..a.len() {
        result += a[i] * b[i];
    }
    result
}
