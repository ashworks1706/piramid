// Quantization primitives for storing vectors in a compressed form.
// Supports scalar int8 quantization (legacy/default) and a lightweight
// product-quantization-style block compressor for better recall/size tradeoffs.

use serde::{Deserialize, Serialize};

use crate::config::QuantizationConfig;

// Tracks which encoding is used; defaults to Scalar so old checkpoints still load.
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
        let block_len = dim.div_ceil(subquantizers);

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
            let (block_min, block_max) = slice
                .iter()
                .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), &v| {
                    (lo.min(v), hi.max(v))
                });
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
        let block_len = self.dim.div_ceil(self.subquantizers);
        let mut idx = 0;

        for block_idx in 0..self.subquantizers {
            let start = block_idx * block_len;
            if start >= self.dim {
                break;
            }
            let end = (start + block_len).min(self.dim);
            let range = (self.block_maxs[block_idx] - self.block_mins[block_idx]).max(f32::EPSILON);

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
            QuantizationKind::Pq => self.pq.as_ref().map(|pq| pq.to_f32()).unwrap_or_else(|| {
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
