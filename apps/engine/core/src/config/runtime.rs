//! Settings re-read after POST /config/reload.
//!
//! A reload takes effect for collections opened after it, and for each request that reads these
//! values on its way through. It does not reopen collections already in memory.

use serde::{Deserialize, Serialize};

use super::{
    CacheConfig, ExecutionMode, IndexConfig, InferenceConfig, LimitsConfig, MemoryConfig,
    QuantizationConfig, QuantizationLevel, QuantizationStage, SearchConfig, WalConfig,
};

/// Everything that can change without a restart.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct RuntimeConfig {
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
    /// Per-collection size ceilings.
    pub limits: LimitsConfig,
    /// Cache sizes and eviction.
    pub cache: CacheConfig,

    /// Which distance-kernel strategy to run.
    pub execution: ExecutionMode,

    /// Model execution.
    pub inference: InferenceConfig,
}

impl RuntimeConfig {
    /// Reject a setting this build cannot honour or a combination that contradicts itself.
    pub fn validate(&self) -> Result<(), String> {
        // compute owns the feature flag and answers for it.
        if matches!(self.execution, ExecutionMode::Gpu)
            && piramid_hardware::compute::strategies::for_mode(ExecutionMode::Gpu).is_err()
        {
            return Err(
                "runtime.execution: 'gpu' requires a build with the `gpu-cuda` feature".into(),
            );
        }
        if matches!(
            self.quantization.level,
            QuantizationLevel::Int4 | QuantizationLevel::Float16
        ) {
            return Err("runtime.quantization.level: not implemented yet".into());
        }
        if self.quantization.level == QuantizationLevel::None
            && self.quantization.stage != QuantizationStage::Disabled
        {
            return Err("runtime.quantization.stage: must be disabled when level is none".into());
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
