// Binary quantization for cosine similarity
// Uses 1-bit quantized vectors for ultra-fast computation
//  Lower precision but 32x memory reduction and much faster

pub fn cosine_similarity_binary(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same length");
    
    //  1 if >= 0, 0 if < 0
    let mut hamming_distance = 0u32;
    let mut count = 0u32;
    
    for i in 0..a.len() {
        let bit_a = if a[i] >= 0.0 { 1u8 } else { 0u8 };
        let bit_b = if b[i] >= 0.0 { 1u8 } else { 0u8 };
        
        // XOR to find differences Hamming distance
        if bit_a != bit_b {
            hamming_distance += 1;
        }
        count += 1;
    }
    
    // Convert Hamming distance to similarity score
    // similarity = 1 - (hamming_distance / total_bits)
    // Then map to [-1, 1] range: 2 * similarity - 1
    let similarity = 1.0 - (hamming_distance as f32 / count as f32);
    2.0 * similarity - 1.0
}
