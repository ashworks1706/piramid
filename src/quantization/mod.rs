// Quantization primitives for storing vectors in a compressed form.
// Supports scalar int8 quantization (legacy/default) and a lightweight
// product-quantization-style block compressor for better recall/size tradeoffs.

use serde::{Deserialize, Serialize};

use crate::config::QuantizationConfig;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum QuantizationKind {
    Scalar,
    Pq,
}

impl QuantizationKind {
    fn scalar() -> Self {
        QuantizationKind::Scalar
    }
}

// Legacy scalar quantization (single min/max for whole vector).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalarQuantizedVector {
    pub values: Vec<i8>,
    pub min: f32,
    pub max: f32,
}

impl ScalarQuantizedVector {
    pub fn from_f32(vector: &[f32]) -> Self {
        if vector.is_empty() {
            return ScalarQuantizedVector {
                values: Vec::new(),
                min: 0.0,
                max: 0.0,
            };
        }

        let min = vector.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        let max = vector.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));

        if (max - min).abs() < f32::EPSILON {
            let values = vec![0i8; vector.len()];
            return ScalarQuantizedVector { values, min, max };
        }

        let range = max - min;
        let quantized_values: Vec<i8> = vector
            .iter()
            .map(|&v| {
                let normalized = (v - min) / range;
                let scaled = normalized * 254.0 - 127.0;
                scaled.round().clamp(-127.0, 127.0) as i8
            })
            .collect();

        ScalarQuantizedVector {
            values: quantized_values,
            min,
            max,
        }
    }

    pub fn to_f32(&self) -> Vec<f32> {
        if self.values.is_empty() {
            return Vec::new();
        }

        if (self.max - self.min).abs() < f32::EPSILON {
            return vec![self.min; self.values.len()];
        }

        let range = self.max - self.min;
        self.values
            .iter()
            .map(|&q| {
                let normalized = (q as f32 + 127.0) / 254.0;
                normalized * range + self.min
            })
            .collect()
    }

    pub fn dim(&self) -> usize {
        self.values.len()
    }
}

// Lightweight PQ representation: store codes and per-block min/max.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductQuantizedVector {
    pub codes: Vec<u8>,
    pub block_mins: Vec<f32>,
    pub block_maxs: Vec<f32>,
    pub dim: usize,
    pub subquantizers: usize,
}

impl ProductQuantizedVector {
    pub fn from_f32(vector: &[f32], subquantizers: usize) -> Self {
        if vector.is_empty() {
            return ProductQuantizedVector {
                codes: Vec::new(),
                block_mins: Vec::new(),
                block_maxs: Vec::new(),
                dim: 0,
                subquantizers: 0,
            };
        }

        let dim = vector.len();
        let subquantizers = subquantizers.max(1).min(dim);
        let block_len = (dim + subquantizers - 1) / subquantizers;

        let mut codes = Vec::with_capacity(dim);
        let mut block_mins = Vec::with_capacity(subquantizers);
        let mut block_maxs = Vec::with_capacity(subquantizers);

        for block_idx in 0..subquantizers {
            let start = block_idx * block_len;
            if start >= dim {
                break;
            }
            let end = (start + block_len).min(dim);
            let slice = &vector[start..end];
            let (block_min, block_max) = slice.iter().fold(
                (f32::INFINITY, f32::NEG_INFINITY),
                |(lo, hi), &v| (lo.min(v), hi.max(v)),
            );
            block_mins.push(block_min);
            block_maxs.push(block_max);

            let range = (block_max - block_min).max(f32::EPSILON);
            for &v in slice {
                let normalized = (v - block_min) / range;
                let code = (normalized * 255.0).round().clamp(0.0, 255.0) as u8;
                codes.push(code);
            }
        }

        ProductQuantizedVector {
            codes,
            block_mins,
            block_maxs,
            dim,
            subquantizers,
        }
    }

    pub fn to_f32(&self) -> Vec<f32> {
        if self.codes.is_empty() || self.subquantizers == 0 {
            return Vec::new();
        }

        let mut values = Vec::with_capacity(self.dim);
        let block_len = (self.dim + self.subquantizers - 1) / self.subquantizers;
        let mut idx = 0;

        for block_idx in 0..self.subquantizers {
            let start = block_idx * block_len;
            if start >= self.dim {
                break;
            }
            let end = (start + block_len).min(self.dim);
            let range =
                (self.block_maxs[block_idx] - self.block_mins[block_idx]).max(f32::EPSILON);

            for _ in start..end {
                let code = self.codes.get(idx).copied().unwrap_or(0);
                let normalized = code as f32 / 255.0;
                values.push(normalized * range + self.block_mins[block_idx]);
                idx += 1;
            }
        }

        values
    }

    pub fn dim(&self) -> usize {
        self.dim
    }
}

// Unified quantized vector. Additional fields default so legacy on-disk data
// (values/min/max only) continues to deserialize correctly under bincode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizedVector {
    pub values: Vec<i8>,
    pub min: f32,
    pub max: f32,
    #[serde(default)]
    pub pq: Option<ProductQuantizedVector>,
    #[serde(default = "QuantizationKind::scalar")]
    pub kind: QuantizationKind,
}

impl QuantizedVector {
    pub fn from_f32(vector: &[f32]) -> Self {
        Self::from_scalar(vector)
    }

    pub fn from_f32_with_config(vector: &[f32], cfg: &QuantizationConfig) -> Self {
        match cfg.level {
            crate::config::QuantizationLevel::Pq { subquantizers } => {
                Self::from_pq(vector, subquantizers)
            }
            _ => Self::from_scalar(vector),
        }
    }

    fn from_scalar(vector: &[f32]) -> Self {
        let scalar = ScalarQuantizedVector::from_f32(vector);
        QuantizedVector {
            values: scalar.values,
            min: scalar.min,
            max: scalar.max,
            pq: None,
            kind: QuantizationKind::Scalar,
        }
    }

    fn from_pq(vector: &[f32], subquantizers: usize) -> Self {
        let pq = ProductQuantizedVector::from_f32(vector, subquantizers);
        QuantizedVector {
            values: Vec::new(),
            min: 0.0,
            max: 0.0,
            pq: Some(pq),
            kind: QuantizationKind::Pq,
        }
    }

    pub fn to_f32(&self) -> Vec<f32> {
        match self.kind {
            QuantizationKind::Scalar => ScalarQuantizedVector {
                values: self.values.clone(),
                min: self.min,
                max: self.max,
            }
            .to_f32(),
            QuantizationKind::Pq => self
                .pq
                .as_ref()
                .map(|pq| pq.to_f32())
                .unwrap_or_else(|| {
                    ScalarQuantizedVector {
                        values: self.values.clone(),
                        min: self.min,
                        max: self.max,
                    }
                    .to_f32()
                }),
        }
    }

    pub fn dim(&self) -> usize {
        match self.kind {
            QuantizationKind::Scalar => self.values.len(),
            QuantizationKind::Pq => self
                .pq
                .as_ref()
                .map(|pq| pq.dim())
                .unwrap_or(self.values.len()),
        }
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
            assert!(
                error < 0.01,
                "Error too large: {} vs {} (diff: {})",
                o,
                d,
                error
            );
        }
    }

    #[test]
    fn test_constant_vector() {
        let original = vec![1.0, 1.0, 1.0, 1.0];
        let quantized = QuantizedVector::from_f32(&original);
        let dequantized = quantized.to_f32();

        for (o, d) in original.iter().zip(dequantized.iter()) {
            assert!(
                (o - d).abs() < 0.001,
                "Original: {}, Dequantized: {}",
                o,
                d
            );
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
        let original = vec![0.123; 1536]; // OpenAI embedding size
        let quantized = QuantizedVector::from_f32(&original);

        let original_size = original.len() * 4;
        let quantized_size = quantized.values.len() + 8;
        let ratio = original_size as f32 / quantized_size as f32;

        assert!(
            ratio > 3.9,
            "Compression ratio should be ~4x, got {}",
            ratio
        );
    }

    #[test]
    fn test_pq_roundtrip() {
        let original: Vec<f32> = (0..32).map(|i| i as f32 * 0.1).collect();
        let pq =
            QuantizedVector::from_f32_with_config(&original, &crate::config::QuantizationConfig::pq(4));
        let restored = pq.to_f32();
        assert_eq!(restored.len(), original.len());
    }
}
