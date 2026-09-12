//! Per-collection configuration: the runtime defaults a collection was created with.

use serde::{Deserialize, Serialize};

use super::{
    CacheConfig, ExecutionMode, HardwareConfig, IndexConfig, LimitsConfig, MemoryConfig,
    QuantizationConfig, SearchConfig, WalConfig,
};

/// The settings one collection runs with.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CollectionConfig {
    /// Index family, metric and family parameters.
    pub index: IndexConfig,
    /// Default search depth and filter overfetch.
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
    /// Cache sizes and eviction.
    pub cache: CacheConfig,
}

impl CollectionConfig {
    /// This configuration with int8 quantization.
    pub fn with_int8_quantization(mut self) -> Self {
        self.quantization = QuantizationConfig::int8();
        self
    }
}
