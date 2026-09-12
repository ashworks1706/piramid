//! The configuration file: blocks split by when a setting takes effect.

use serde::{Deserialize, Serialize};

use super::{CollectionConfig, ConsoleConfig, RuntimeConfig, StartupConfig};

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
    /// Validate every block and reject a GPU profile paired with any execution mode but gpu.
    pub fn validate(&self) -> Result<(), String> {
        self.startup.validate()?;
        self.runtime.validate()?;
        self.console.validate()?;
        if self.startup.hardware.gpu_enabled()
            && self.runtime.execution != super::ExecutionMode::Gpu
        {
            return Err(format!(
                "startup.hardware.profile: gpu requires runtime.execution: gpu, not '{}'",
                self.runtime.execution.as_str()
            ));
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
