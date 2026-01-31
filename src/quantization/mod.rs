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
        let min = vector.iter().clone().fold(f32::INFINITY, |a, &b| a.min(b)); // iterates through
        // through vector by reference and then clones the iterator to avoid borrowing issues, then
        // it performs a fold operation --- fold takes an initial value (f32::INFINITY) and a
        // closure that compares each element to the accumulator (a) and returns the minimum value
        // found.
        let max = vector.iter().clone().fold(f32::INFINITY, |a, &b| a.max(b));

        // calculate scale factor
        let scale = if max - min == 0.0 { 1.0 } else { 255.0 / (max - min) };
        let quantized_values = vector.iter().map(|&v| {
            let q = ((v - min) * scale).round() as i8;
            q
        }).collect();

        QuantizedVector {
            values: quantized_values,
            min,
            max,
        }
    }

    // dequantize back to f32 
    pub fn to_f32(&self) -> Vec<f32> {
        let scale = if self.max - self.min == 0.0 { 1.0 } else { (self.max - self.min) / 255.0 };
        self.values.iter().map(|&q| {
            (q as f32) * scale + self.min
        }).collect()
    }

    // get dimensionality
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
            assert!((o - d).abs() < 0.1, "Original: {}, Dequantized: {}", o, d);
        }
    }
    #[test]
    fn test_zero_vector() {
        let original = vec![1.0, 1.0, 1.0, 1.0];
        let quantized = QuantizedVector::from_f32(&original);
        let dequantized = quantized.to_f32();
        for (o, d) in original.iter().zip(dequantized.iter()) {
            assert!((o - d).abs() < 0.1, "Original:
    {}, Dequantized: {}", o, d);
        }
    }
}
