//! Vector compression settings.

use serde::{Deserialize, Serialize};

/// Which compression a collection asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum QuantizationLevel {
    /// Full-precision f32.
    #[default]
    None,
    /// 8-bit integer, scaled per vector.
    Int8,
    /// Product quantization with a fixed number of blocks.
    Pq {
        /// Number of blocks the vector is split into.
        subquantizers: usize,
    },
    /// 4-bit integer. Not implemented; rejected by runtime config validation.
    Int4,
    /// Half precision. Not implemented; rejected by runtime config validation.
    Float16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
/// Which point in the pipeline quantization applies at.
pub enum QuantizationStage {
    /// No quantization anywhere.
    #[default]
    Disabled,
    /// Quantize what is written to disk.
    Storage,
    /// Quantize what the index scores against.
    Index,
    /// Quantize the query before searching.
    QueryPreSearch,
    /// Quantize results after searching.
    ResultPostSearch,
}

/// Where and how aggressively vectors are compressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuantizationConfig {
    /// The encoding to use.
    pub level: QuantizationLevel,

    /// Compress on disk only. When false, the in-memory copy is quantized as well.
    pub disk_only: bool,

    /// Where in the pipeline the encoding applies.
    #[serde(default)]
    pub stage: QuantizationStage,

    /// Keep full-precision vectors alongside the quantized copies.
    #[serde(default = "default_preserve_raw_vectors")]
    pub preserve_raw_vectors: bool,
}

impl Default for QuantizationConfig {
    fn default() -> Self {
        QuantizationConfig {
            level: QuantizationLevel::None,
            disk_only: false,
            stage: QuantizationStage::Disabled,
            preserve_raw_vectors: true,
        }
    }
}

impl QuantizationConfig {
    /// Quantize for index and search while keeping raw vectors on disk.
    pub fn int8() -> Self {
        QuantizationConfig {
            level: QuantizationLevel::Int8,
            disk_only: false,
            stage: QuantizationStage::Index,
            preserve_raw_vectors: true,
        }
    }

    /// Product quantization for the index, raw vectors kept on disk.
    pub fn pq(subquantizers: usize) -> Self {
        QuantizationConfig {
            level: QuantizationLevel::Pq { subquantizers },
            disk_only: false,
            stage: QuantizationStage::Index,
            preserve_raw_vectors: true,
        }
    }
}

fn default_preserve_raw_vectors() -> bool {
    true
}
