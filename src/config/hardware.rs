use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum HardwareProfile {
    #[default]
    Auto,
    CpuOnly,
    Gpu,
    Memory8Gb,
    Memory16Gb,
    Memory32Gb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardwareConfig {
    #[serde(default)]
    pub profile: HardwareProfile,
    #[serde(default)]
    pub cpu_threads: Option<usize>,
    #[serde(default)]
    pub memory_budget_bytes: Option<u64>,
    #[serde(default)]
    pub gpu_enabled: bool,
    #[serde(default)]
    pub gpu_memory_budget_bytes: Option<u64>,
}

impl Default for HardwareConfig {
    fn default() -> Self {
        Self {
            profile: HardwareProfile::Auto,
            cpu_threads: None,
            memory_budget_bytes: None,
            gpu_enabled: false,
            gpu_memory_budget_bytes: None,
        }
    }
}

impl HardwareConfig {
    pub fn cpu_only() -> Self {
        Self {
            profile: HardwareProfile::CpuOnly,
            gpu_enabled: false,
            ..Self::default()
        }
    }

    pub fn gpu() -> Self {
        Self {
            profile: HardwareProfile::Gpu,
            gpu_enabled: true,
            ..Self::default()
        }
    }
}
