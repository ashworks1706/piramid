//! What the machine has, and how much of it to use.

use serde::{Deserialize, Serialize};

/// Which hardware to run on, and at what memory class.
///
/// The memory-class profiles set the host memory budget when memory_budget_bytes is unset.
/// Validation refuses them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum HardwareProfile {
    /// Never touch the GPU.
    #[default]
    CpuOnly,
    /// Require a GPU; fail to start without one.
    Gpu,
    /// A small host with 8 GB of memory.
    #[serde(rename = "8gb")]
    Memory8Gb,
    /// A mid-sized host.
    #[serde(rename = "16gb")]
    Memory16Gb,
    /// A large host: room to keep vectors resident at full precision.
    #[serde(rename = "32gb")]
    Memory32Gb,
}

impl HardwareProfile {
    /// Host memory this profile assumes, when it names one.
    ///
    /// None for cpu-only and gpu.
    pub fn memory_class_bytes(&self) -> Option<u64> {
        const GB: u64 = 1024 * 1024 * 1024;
        match self {
            HardwareProfile::Memory8Gb => Some(8 * GB),
            HardwareProfile::Memory16Gb => Some(16 * GB),
            HardwareProfile::Memory32Gb => Some(32 * GB),
            HardwareProfile::CpuOnly | HardwareProfile::Gpu => None,
        }
    }

    /// Stable lowercase name, matching the serde representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            HardwareProfile::CpuOnly => "cpu-only",
            HardwareProfile::Gpu => "gpu",
            HardwareProfile::Memory8Gb => "8gb",
            HardwareProfile::Memory16Gb => "16gb",
            HardwareProfile::Memory32Gb => "32gb",
        }
    }
}

/// Hardware selection and memory budgets for the process.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct HardwareConfig {
    /// Which hardware to run on, or which memory class to assume.
    pub profile: HardwareProfile,

    /// Host memory the process will use. None takes the profile's memory class, or is unbounded.
    pub memory_budget_bytes: Option<u64>,

    /// Device memory to claim, before gpu.reserve_bytes is held back. None is the whole device.
    pub gpu_memory_budget_bytes: Option<u64>,

    /// Device selection and kernel launch shapes.
    pub gpu: GpuConfig,

    /// Division of device memory between weights, KV cache and retrieval.
    pub vram: VramSplit,
}

impl HardwareConfig {
    /// Whether to acquire a device. The profile is the only switch.
    pub fn gpu_enabled(&self) -> bool {
        matches!(self.profile, HardwareProfile::Gpu)
    }

    /// Host memory ceiling: the explicit setting, else whatever the profile's class implies.
    pub fn memory_budget(&self) -> Option<u64> {
        self.memory_budget_bytes
            .or_else(|| self.profile.memory_class_bytes())
    }
}

/// Device selection and the shapes kernels launch with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct GpuConfig {
    /// Which device to open when more than one is present.
    pub device_ordinal: usize,

    /// Threads per block for distance kernels.
    pub distance_block_size: u32,

    /// Independent streams opened with the device. Retrieval kernels queue on the first; the
    /// model queues on the per-thread stream.
    pub streams: usize,

    /// Device bytes held back for fragmentation and library workspaces.
    pub reserve_bytes: u64,
}

impl Default for GpuConfig {
    fn default() -> Self {
        GpuConfig {
            device_ordinal: 0,
            distance_block_size: 256,
            streams: 2,
            reserve_bytes: 1024 * 1024 * 1024,
        }
    }
}

/// How device memory is divided between model weights, the KV cache and retrieval.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct VramSplit {
    /// Enforce the split. Off means every pool draws from one budget, first come first served.
    pub enabled: bool,

    /// Share for model weights.
    pub weights_ratio: f32,

    /// Share for the KV cache.
    pub kv_ratio: f32,

    /// Share for stored vectors and scoring buffers on the device.
    pub vectors_ratio: f32,

    /// Fraction of retrieval-side bandwidth to allow while a forward pass is decoding.
    pub retrieval_bandwidth_share: f32,
}

impl Default for VramSplit {
    fn default() -> Self {
        VramSplit {
            enabled: false,
            weights_ratio: 0.6,
            kv_ratio: 0.15,
            vectors_ratio: 0.25,
            retrieval_bandwidth_share: 0.25,
        }
    }
}

impl VramSplit {
    /// Reject a split that cannot be honoured.
    pub fn validate(&self) -> Result<(), String> {
        if self.retrieval_bandwidth_share != VramSplit::default().retrieval_bandwidth_share {
            return Err(
                "startup.hardware.vram.retrieval_bandwidth_share: not implemented yet (roadmap v0.6.0)"
                    .into(),
            );
        }
        if !self.enabled {
            return Ok(());
        }
        let shares = [self.weights_ratio, self.kv_ratio, self.vectors_ratio];
        if shares.iter().any(|share| !(0.0..=1.0).contains(share)) {
            return Err("startup.hardware.vram: each ratio must be within 0.0..=1.0".into());
        }
        if shares.iter().sum::<f32>() > 1.0 + 1e-6 {
            return Err("startup.hardware.vram: the ratios must sum to at most 1.0".into());
        }
        Ok(())
    }
}

impl GpuConfig {
    /// Reject anything the build cannot honour.
    pub fn validate(&self) -> Result<(), String> {
        if self.distance_block_size == 0 || !self.distance_block_size.is_multiple_of(32) {
            return Err(
                "startup.hardware.gpu.distance_block_size: must be a non-zero multiple of 32"
                    .into(),
            );
        }
        if self.streams == 0 {
            return Err("startup.hardware.gpu.streams: must be >= 1".into());
        }
        Ok(())
    }
}
