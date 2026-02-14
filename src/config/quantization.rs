// Quantization configuration for vector compression

use serde::{Deserialize, Serialize};

// Quantization level for vector compression
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuantizationLevel {
    // No quantization - full precision float32
    None,
    // 8-bit integer quantization (4x memory reduction)
    Int8,
    // Product quantization (block-wise min/max compression)
    Pq { subquantizers: usize },
    // 4-bit integer quantization (8x memory reduction) - Future
    Int4,
    // 16-bit float quantization (2x memory reduction) - Future
    Float16,
}

impl Default for QuantizationLevel {
    fn default() -> Self {
        QuantizationLevel::None
    }
}

// Quantization configuration
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct QuantizationConfig {
    // Quantization level to use
    pub level: QuantizationLevel,
    
    // Whether to compress vectors on disk only (false = also in memory)
    pub disk_only: bool,
}

impl Default for QuantizationConfig {
    fn default() -> Self {
        QuantizationConfig {
            level: QuantizationLevel::None,
            disk_only: false,
        }
    }
}

impl QuantizationConfig {
    // Enable int8 quantization
    pub fn int8() -> Self {
        QuantizationConfig {
            level: QuantizationLevel::Int8,
            disk_only: false,
        }
    }
    
    // Enable int8 quantization for disk only
    pub fn int8_disk_only() -> Self {
        QuantizationConfig {
            level: QuantizationLevel::Int8,
            disk_only: true,
        }
    }

    // Enable CPU product quantization with the given number of subquantizers.
    pub fn pq(subquantizers: usize) -> Self {
        QuantizationConfig {
            level: QuantizationLevel::Pq { subquantizers },
            disk_only: false,
        }
    }
}
