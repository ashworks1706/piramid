// Binary quantization for Euclidean distance
// Uses 1-bit quantized vectors

pub fn euclidean_distance_binary(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same length");

    let mut hamming_distance = 0u32;

    for i in 0..a.len() {
        let bit_a = if a[i] >= 0.0 { 1u8 } else { 0u8 };
        let bit_b = if b[i] >= 0.0 { 1u8 } else { 0u8 };

        if bit_a != bit_b {
            hamming_distance += 1;
        }
    }

    // Approximate Euclidean distance from Hamming distance
    (hamming_distance as f32).sqrt()
}

pub fn euclidean_distance_squared_binary(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same length");

    let mut hamming_distance = 0u32;

    for i in 0..a.len() {
        let bit_a = if a[i] >= 0.0 { 1u8 } else { 0u8 };
        let bit_b = if b[i] >= 0.0 { 1u8 } else { 0u8 };

        if bit_a != bit_b {
            hamming_distance += 1;
        }
    }

    hamming_distance as f32
}
