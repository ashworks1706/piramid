//! Settings re-read after POST /config/reload.
//!
//! A reload applies search, limits, WAL checkpoint thresholds, the metadata cache budget and the
//! execution mode to collections already open. A change to a setting read only when a collection
//! opens refuses the reload while any collection is open.

use serde::{Deserialize, Serialize};

use super::{
    CacheConfig, ExecutionMode, IndexConfig, InferenceConfig, LimitsConfig, MemoryConfig,
    QuantizationConfig, SearchConfig, WalConfig,
};

/// Everything that can change without a restart.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct RuntimeConfig {
    pub index: IndexConfig,
    pub search: SearchConfig,
    pub quantization: QuantizationConfig,
    pub memory: MemoryConfig,
    pub wal: WalConfig,
    pub limits: LimitsConfig,
    pub cache: CacheConfig,

    /// Which distance-kernel strategy to run.
    pub execution: ExecutionMode,

    pub inference: InferenceConfig,
}

impl RuntimeConfig {
    pub fn validate(&self) -> Result<(), String> {
        // compute answers for whether a strategy can run.
        if let Err(error) = piramid_hardware::compute::strategies::for_mode(self.execution) {
            return Err(format!("runtime.execution: {error}"));
        }
        if self.quantization != QuantizationConfig::default() {
            return Err(
                "runtime.quantization: nothing applies quantization yet, so every key must stay \
                 at its default"
                    .into(),
            );
        }
        if self.memory.max_memory_per_collection.is_some() {
            return Err("runtime.memory.max_memory_per_collection: not enforced yet".into());
        }
        if self.wal.enabled && self.wal.checkpoint_frequency == 0 {
            return Err("runtime.wal.checkpoint_frequency: must be > 0 when the WAL is on".into());
        }
        if self.search.filter_overfetch == 0 {
            return Err("runtime.search.filter_overfetch: must be >= 1".into());
        }
        if self.memory.use_mmap && self.memory.initial_mmap_size == 0 {
            return Err("runtime.memory.initial_mmap_size: must be > 0 when mmap is on".into());
        }
        self.cache.validate()?;
        self.index.validate()?;
        self.inference.validate()
    }
}
