//! Per-collection configuration: the runtime defaults a collection was created with.

use serde::{Deserialize, Serialize};

use super::{
    CacheConfig, ExecutionMode, HardwareConfig, IndexConfig, LimitsConfig, MemoryConfig,
    QuantizationConfig, SearchConfig, WalConfig,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CollectionConfig {
    pub index: IndexConfig,
    pub search: SearchConfig,
    pub quantization: QuantizationConfig,
    pub memory: MemoryConfig,
    pub wal: WalConfig,
    pub execution: ExecutionMode,
    pub hardware: HardwareConfig,
    pub limits: LimitsConfig,
    pub cache: CacheConfig,
}
