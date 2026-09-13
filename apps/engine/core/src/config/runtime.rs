//! Settings re-read after POST /config/reload.

use serde::{Deserialize, Serialize};

use piramid_hardware::compute::ExecutionMode;

use super::{
    InferenceConfig, LimitsConfig, MemoryConfig, QuantizationConfig, SearchConfig, WalConfig,
};

/// Everything that can change without a restart.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct RuntimeConfig {
    /// Metric for a new collection, and batch search parallelism.
    pub search: SearchConfig,
    /// How stored vectors are compressed.
    pub quantization: QuantizationConfig,
    /// Memory ceiling and mapping of the data file.
    pub memory: MemoryConfig,
    /// Write-ahead log and checkpoint cadence.
    pub wal: WalConfig,
    /// Per-collection size ceilings.
    pub limits: LimitsConfig,

    /// Which distance-kernel strategy to run.
    pub execution: ExecutionMode,

    /// Model execution.
    pub inference: InferenceConfig,
}

impl RuntimeConfig {
    /// Reject a setting this build cannot honour or a combination that contradicts itself.
    pub fn validate(&self) -> Result<(), String> {
        piramid_hardware::compute::strategies::compiled(self.execution)
            .map_err(|error| format!("runtime.execution: {error}"))?;
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
        if self.memory.use_mmap && self.memory.initial_mmap_size == 0 {
            return Err("runtime.memory.initial_mmap_size: must be > 0 when mmap is on".into());
        }
        self.inference.validate()
    }
}
