//! Per-collection configuration: the runtime defaults a collection was created with.

use serde::{Deserialize, Serialize};

use piramid_hardware::compute::ExecutionMode;

use super::{
    HardwareConfig, LimitsConfig, MemoryConfig, QuantizationConfig, SearchConfig, WalConfig,
};

/// The settings one collection runs with.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CollectionConfig {
    /// Metric for a new collection, and batch search parallelism.
    pub search: SearchConfig,
    /// How stored vectors are compressed.
    pub quantization: QuantizationConfig,
    /// Memory ceiling and mapping of the data file.
    pub memory: MemoryConfig,
    /// Write-ahead log and checkpoint cadence.
    pub wal: WalConfig,
    /// Distance strategy used for scoring.
    pub execution: ExecutionMode,
    /// Hardware settings copied from the startup block.
    pub hardware: HardwareConfig,
    /// Per-collection size ceilings.
    pub limits: LimitsConfig,
}
