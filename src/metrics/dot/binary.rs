// Binary quantization for dot product

pub fn dot_product_binary(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same length");

    let mut count_both_positive = 0u32;

    for i in 0..a.len() {
        let bit_a = if a[i] >= 0.0 { 1u8 } else { 0u8 };
        let bit_b = if b[i] >= 0.0 { 1u8 } else { 0u8 };

        if bit_a == 1 && bit_b == 1 {
            count_both_positive += 1;
        }
    }

    // Approximate dot product from bit agreement
    count_both_positive as f32
}
