// Per-collection configuration that combines all settings

use serde::{Deserialize, Serialize};
use super::*;

// Unified collection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionConfig {
    // Index configuration
    pub index: crate::index::IndexConfig,
    
    // Quantization settings
    #[serde(default)]
    pub quantization: QuantizationConfig,
    
    // Memory limits
    #[serde(default)]
    pub memory: MemoryConfig,
    
    // WAL settings
    #[serde(default)]
    pub wal: WalConfig,
    
    // Parallelism settings
    #[serde(default)]
    pub parallelism: ParallelismConfig,
}

impl Default for CollectionConfig {
    fn default() -> Self {
        CollectionConfig {
            index: crate::index::IndexConfig::default(),
            quantization: QuantizationConfig::default(),
            memory: MemoryConfig::default(),
            wal: WalConfig::default(),
            parallelism: ParallelismConfig::default(),
        }
    }
}

impl CollectionConfig {
    // Create a new config with custom index
    pub fn with_index(index: crate::index::IndexConfig) -> Self {
        CollectionConfig {
            index,
            ..Default::default()
        }
    }
    
    // Enable int8 quantization
    pub fn with_int8_quantization(mut self) -> Self {
        self.quantization = QuantizationConfig::int8();
        self
    }
    
    // Set memory limit in MB
    pub fn with_memory_limit_mb(mut self, limit_mb: usize) -> Self {
        self.memory = MemoryConfig::with_limit_mb(limit_mb);
        self
    }
    
    // Disable WAL
    pub fn without_wal(mut self) -> Self {
        self.wal = WalConfig::disabled();
        self
    }
    
    // Use single-threaded mode
    pub fn single_threaded(mut self) -> Self {
        self.parallelism = ParallelismConfig::single_threaded();
        self
    }
}
