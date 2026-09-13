//! The configuration file: blocks split by when a setting takes effect.

use piramid_hardware::compute::ExecutionMode;
use serde::{Deserialize, Serialize};

use super::{CollectionConfig, ConsoleConfig, DeviceSelection, RuntimeConfig, StartupConfig};

/// The whole of config.yaml.
///
/// [StartupConfig] is baked into the process at boot, [RuntimeConfig] is re-read on reload, and
/// [ConsoleConfig] is read by the terminal UI when it starts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct Config {
    /// Settings applied once at boot.
    pub startup: StartupConfig,
    /// Settings re-read on reload.
    pub runtime: RuntimeConfig,
    /// Settings for the terminal UI.
    pub console: ConsoleConfig,
}

impl Config {
    /// Validate every block, and reject a GPU profile paired with any execution mode but gpu, gpu
    /// execution without the GPU profile, and an inference device the profile does not open.
    pub fn validate(&self) -> Result<(), String> {
        if let Some(Ok(DeviceSelection::Cuda(ordinal))) = self
            .runtime
            .inference
            .device
            .as_deref()
            .map(DeviceSelection::parse)
        {
            if !self.startup.hardware.gpu_enabled() {
                return Err(
                    "runtime.inference.device: a cuda device needs startup.hardware.profile: gpu"
                        .into(),
                );
            }
            if ordinal != self.startup.hardware.gpu.device_ordinal {
                return Err(format!(
                    "runtime.inference.device: must be cuda:{}, the device startup.hardware.gpu.device_ordinal opens",
                    self.startup.hardware.gpu.device_ordinal
                ));
            }
        }
        self.startup.validate()?;
        self.runtime.validate()?;
        self.console.validate()?;
        if self.startup.hardware.gpu_enabled() && self.runtime.execution != ExecutionMode::Gpu {
            return Err(format!(
                "startup.hardware.profile: gpu requires runtime.execution: gpu, not '{}'",
                self.runtime.execution.as_str()
            ));
        }
        if self.runtime.execution == ExecutionMode::Gpu && !self.startup.hardware.gpu_enabled() {
            return Err(
                "runtime.execution: gpu needs startup.hardware.profile: gpu to open a device"
                    .into(),
            );
        }
        Ok(())
    }

    /// The defaults a newly created collection inherits.
    pub fn to_collection_config(&self) -> CollectionConfig {
        CollectionConfig {
            index: self.runtime.index.clone(),
            search: self.runtime.search,
            quantization: self.runtime.quantization,
            memory: self.runtime.memory,
            wal: self.runtime.wal,
            execution: self.runtime.execution,
            hardware: self.startup.hardware,
            limits: self.runtime.limits,
            cache: self.runtime.cache,
        }
    }
}
