// Scalar quantization for memory efficient storage of vectors. 


use serde::{Serialize, Deserialize};

// stores vectors as 8bit integer with min/max metadata for reconstruction 
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizedVector{
    pub values: Vec<i8>, // these are quantized values 
    pub min:f32, // these are original values 
    pub max: f32,
}

impl QuantizedVector {
    pub fn from_f32(vector: &[f32]) -> Self {
        if vector.is_empty() {
            return QuantizedVector {
                values: Vec::new(),
                min: 0.0,
                max: 0.0,
            };
        }

        let min = vector.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        let max = vector.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));

        // Handle constant vectors (all same value)
        if (max - min).abs() < f32::EPSILON {
            let values = vec![0i8; vector.len()];
            return QuantizedVector { values, min, max };
        }

        // Quantize to [-127, 127] range (254 discrete values)
        let range = max - min;
        let quantized_values: Vec<i8> = vector
            .iter()
            .map(|&v| {
                let normalized = (v - min) / range;  // 0.0 to 1.0
                let scaled = normalized * 254.0 - 127.0;  // -127.0 to 127.0
                scaled.round().clamp(-127.0, 127.0) as i8
            })
            .collect();

        QuantizedVector {
            values: quantized_values,
            min,
            max,
        }
    }

    // Dequantize back to f32
    pub fn to_f32(&self) -> Vec<f32> {
        if self.values.is_empty() {
            return Vec::new();
        }

        // Handle constant vectors
        if (self.max - self.min).abs() < f32::EPSILON {
            return vec![self.min; self.values.len()];
        }

        let range = self.max - self.min;
        self.values
            .iter()
            .map(|&q| {
                let normalized = (q as f32 + 127.0) / 254.0;  // 0.0 to 1.0
                normalized * range + self.min
            })
            .collect()
    }

    // Get dimensionality
    pub fn dim(&self) -> usize {
        self.values.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantization() {
        let original = vec![0.0, 0.5, 1.0, 1.5, 2.0];
        let quantized = QuantizedVector::from_f32(&original);
        let dequantized = quantized.to_f32();
        
        for (o, d) in original.iter().zip(dequantized.iter()) {
            let error = (o - d).abs();
            assert!(error < 0.01, "Error too large: {} vs {} (diff: {})", o, d, error);
        }
    }
    
    #[test]
    fn test_constant_vector() {
        let original = vec![1.0, 1.0, 1.0, 1.0];
        let quantized = QuantizedVector::from_f32(&original);
        let dequantized = quantized.to_f32();
        
        for (o, d) in original.iter().zip(dequantized.iter()) {
            assert!((o - d).abs() < 0.001, "Original: {}, Dequantized: {}", o, d);
        }
    }

    #[test]
    fn test_negative_values() {
        let original = vec![-1.0, -0.5, 0.0, 0.5, 1.0];
        let quantized = QuantizedVector::from_f32(&original);
        let dequantized = quantized.to_f32();
        
        for (o, d) in original.iter().zip(dequantized.iter()) {
            let error = (o - d).abs();
            assert!(error < 0.01, "Error too large: {} vs {}", o, d);
        }
    }

    #[test]
    fn test_memory_reduction() {
        let original = vec![0.123; 1536];  // OpenAI embedding size
        let quantized = QuantizedVector::from_f32(&original);
        
        // Original: 1536 * 4 bytes = 6144 bytes
        // Quantized: 1536 * 1 byte + 8 bytes overhead = 1544 bytes
        // Ratio: 6144 / 1544 = 3.97x reduction
        
        let original_size = original.len() * 4;
        let quantized_size = quantized.values.len() + 8;
        let ratio = original_size as f32 / quantized_size as f32;
        
        assert!(ratio > 3.9, "Compression ratio should be ~4x, got {}", ratio);
    }
}
