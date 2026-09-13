//! Compressed vector representations.

use serde::{Deserialize, Serialize};

use crate::compute::error::{ComputeError, ComputeResult};

/// Which encoding a stored vector uses.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum QuantizationKind {
    /// One min/max pair for the whole vector.
    Scalar,
    /// A code per block; see [ProductQuantizedVector].
    Pq,
}

/// One min/max pair for the whole vector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalarQuantizedVector {
    /// One quantized code per dimension.
    pub values: Vec<i8>,
    /// Smallest value in the original vector.
    pub min: f32,
    /// Largest value in the original vector.
    pub max: f32,
}

impl ScalarQuantizedVector {
    /// Quantize to one code per dimension over a single min/max range.
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

    /// Reconstruct the approximate original values.
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
                let normalized = (f32::from(q) + 127.0) / 254.0;
                normalized * range + self.min
            })
            .collect()
    }
}

/// Per-block codes with their own min/max pairs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductQuantizedVector {
    /// One code per dimension, block by block.
    pub codes: Vec<u8>,
    /// Smallest value per block.
    pub block_mins: Vec<f32>,
    /// Largest value per block.
    pub block_maxs: Vec<f32>,
    /// Original vector width.
    pub dim: usize,
    /// Number of blocks the vector was split into.
    pub subquantizers: usize,
}

impl ProductQuantizedVector {
    /// Quantizes block by block into the requested number of blocks, erroring on a bad count.
    pub fn from_f32(vector: &[f32], subquantizers: usize) -> ComputeResult<Self> {
        if vector.is_empty() {
            return Ok(ProductQuantizedVector {
                codes: Vec::new(),
                block_mins: Vec::new(),
                block_maxs: Vec::new(),
                dim: 0,
                subquantizers: 0,
            });
        }

        let dim = vector.len();
        if subquantizers == 0 || subquantizers > dim {
            return Err(ComputeError::InvalidEncoding(format!(
                "pq subquantizers {subquantizers} must be between 1 and the dimension {dim}"
            )));
        }
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

        Ok(ProductQuantizedVector {
            codes,
            block_mins,
            block_maxs,
            dim,
            subquantizers,
        })
    }

    /// Decode, erroring when the encoding is internally inconsistent.
    pub fn to_f32(&self) -> ComputeResult<Vec<f32>> {
        if self.codes.is_empty() || self.subquantizers == 0 {
            if self.dim == 0 {
                return Ok(Vec::new());
            }
            return Err(ComputeError::InvalidEncoding(
                "PQ vector has no codes or subquantizers for non-empty dimension".into(),
            ));
        }
        if self.block_mins.len() < self.subquantizers || self.block_maxs.len() < self.subquantizers
        {
            return Err(ComputeError::InvalidEncoding(
                "PQ vector block metadata is shorter than subquantizer count".into(),
            ));
        }

        let mut values = Vec::with_capacity(self.dim);
        let block_len = self.dim.div_ceil(self.subquantizers);

        for block_idx in 0..self.subquantizers {
            let start = block_idx * block_len;
            if start >= self.dim {
                break;
            }
            let end = (start + block_len).min(self.dim);
            let range = (self.block_maxs[block_idx] - self.block_mins[block_idx]).max(f32::EPSILON);
            let block_min = self.block_mins[block_idx];

            let codes = self.codes.get(start..end).ok_or_else(|| {
                ComputeError::InvalidEncoding(format!(
                    "PQ vector missing codes for block {block_idx}"
                ))
            })?;
            for &code in codes {
                let normalized = f32::from(code) / 255.0;
                values.push(normalized * range + block_min);
            }
        }

        if values.len() != self.dim {
            return Err(ComputeError::InvalidEncoding(format!(
                "PQ vector decoded dimension mismatch: expected {}, got {}",
                self.dim,
                values.len()
            )));
        }

        Ok(values)
    }

    /// Original vector width.
    pub fn dim(&self) -> usize {
        self.dim
    }
}

/// A stored vector in whichever encoding was configured when it was written.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizedVector {
    /// Scalar codes; empty for a PQ vector.
    pub values: Vec<i8>,
    /// Scalar range minimum.
    pub min: f32,
    /// Scalar range maximum.
    pub max: f32,
    /// The PQ payload, when kind is [QuantizationKind::Pq].
    pub pq: Option<ProductQuantizedVector>,
    /// Which encoding the values and pq fields hold.
    pub kind: QuantizationKind,
}

impl QuantizedVector {
    /// Quantize to one code per dimension over a single min/max range.
    pub fn scalar(vector: &[f32]) -> Self {
        let scalar = ScalarQuantizedVector::from_f32(vector);
        QuantizedVector {
            values: scalar.values,
            min: scalar.min,
            max: scalar.max,
            pq: None,
            kind: QuantizationKind::Scalar,
        }
    }

    /// Quantizes block by block into subquantizers blocks, erroring on a bad count.
    pub fn pq(vector: &[f32], subquantizers: usize) -> ComputeResult<Self> {
        let pq = ProductQuantizedVector::from_f32(vector, subquantizers)?;
        Ok(QuantizedVector {
            values: Vec::new(),
            min: 0.0,
            max: 0.0,
            pq: Some(pq),
            kind: QuantizationKind::Pq,
        })
    }

    /// Decode, erroring when the encoding is internally inconsistent.
    pub fn to_f32(&self) -> ComputeResult<Vec<f32>> {
        match self.kind {
            QuantizationKind::Scalar => Ok(ScalarQuantizedVector {
                values: self.values.clone(),
                min: self.min,
                max: self.max,
            }
            .to_f32()),
            QuantizationKind::Pq => {
                let pq = self.pq.as_ref().ok_or_else(|| {
                    ComputeError::InvalidEncoding(
                        "vector is marked as PQ but has no PQ payload".into(),
                    )
                })?;
                pq.to_f32()
            }
        }
    }

    /// Width of the vector this encodes, or None if the encoding is inconsistent.
    pub fn dim(&self) -> Option<usize> {
        match self.kind {
            QuantizationKind::Scalar => Some(self.values.len()),
            QuantizationKind::Pq => self.pq.as_ref().map(ProductQuantizedVector::dim),
        }
    }
}
