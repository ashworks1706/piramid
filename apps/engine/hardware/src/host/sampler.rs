//! Takes host readings from the operating system.

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

use crate::host::reading::{GpuReading, HostReading};

/// Reads host processor and memory use; the first sample carries no processor fields.
#[derive(Debug)]
pub struct HostSampler {
    system: System,
    pid: Option<Pid>,
    primed: bool,
}

impl HostSampler {
    /// The shortest span between samples that gives an accurate processor reading.
    pub const MINIMUM_INTERVAL: std::time::Duration = sysinfo::MINIMUM_CPU_UPDATE_INTERVAL;

    /// A sampler with no samples taken.
    pub fn new() -> Self {
        Self {
            system: System::new(),
            pid: sysinfo::get_current_pid().ok(),
            primed: false,
        }
    }

    /// Take one reading.
    pub fn sample(&mut self) -> HostReading {
        if !sysinfo::IS_SUPPORTED_SYSTEM {
            return HostReading::default();
        }
        let primed = std::mem::replace(&mut self.primed, true);

        self.system.refresh_memory();
        let memory_total_bytes = Some(self.system.total_memory()).filter(|total| *total > 0);
        let memory_used_bytes = memory_total_bytes.map(|_| self.system.used_memory());

        self.system.refresh_cpu_usage();
        let cpus = self.system.cpus().len();
        let measured = primed && cpus > 0;
        let cpu_percent = measured.then(|| self.system.global_cpu_usage());

        let (process_cpu_percent, process_resident_bytes) = match self.pid {
            Some(pid) => {
                self.system.refresh_processes_specifics(
                    ProcessesToUpdate::Some(&[pid]),
                    true,
                    ProcessRefreshKind::nothing()
                        .with_cpu()
                        .with_memory()
                        .without_tasks(),
                );
                match self.system.process(pid) {
                    Some(process) => (
                        measured.then(|| share_of_host(process.cpu_usage(), cpus)),
                        Some(process.memory()),
                    ),
                    None => (None, None),
                }
            }
            None => (None, None),
        };

        HostReading {
            cpu_percent,
            memory_used_bytes,
            memory_total_bytes,
            process_cpu_percent,
            process_resident_bytes,
        }
    }
}

impl Default for HostSampler {
    fn default() -> Self {
        Self::new()
    }
}

/// Converts a percentage of one logical CPU into a percentage of all of them.
pub fn share_of_host(percent_of_one_cpu: f32, cpus: usize) -> f32 {
    percent_of_one_cpu / cpus as f32
}

/// Reads memory, utilisation and temperature of every GPU the driver reports.
#[derive(Debug)]
pub struct GpuSampler {
    #[cfg(feature = "gpu-cuda")]
    library: Option<crate::host::nvml::Library>,
    #[cfg(feature = "gpu-cuda")]
    reported: bool,
}

impl GpuSampler {
    /// A sampler bound to the driver library, when this build and this machine have one.
    #[cfg(feature = "gpu-cuda")]
    pub fn new() -> Self {
        let library = match crate::host::nvml::Library::load() {
            Ok(library) => Some(library),
            Err(reason) => {
                tracing::warn!(
                    target: "piramid::host",
                    %reason,
                    "GPU readings are absent: the NVIDIA Management Library did not load"
                );
                None
            }
        };
        Self {
            library,
            reported: false,
        }
    }

    /// A sampler that reads no GPU, as this build has no driver library.
    #[cfg(not(feature = "gpu-cuda"))]
    pub fn new() -> Self {
        tracing::debug!(
            target: "piramid::host",
            "GPU readings are absent: built without the gpu-cuda feature"
        );
        Self {}
    }

    /// Take one reading per device. Empty when no device is measured.
    #[cfg(feature = "gpu-cuda")]
    pub fn sample(&mut self) -> Vec<GpuReading> {
        let Some(library) = &self.library else {
            return Vec::new();
        };
        match library.sample() {
            Ok(readings) if !readings.is_empty() => readings,
            Ok(_) => {
                self.report_absent("the driver reports no device");
                Vec::new()
            }
            Err(reason) => {
                self.report_absent(&reason);
                Vec::new()
            }
        }
    }

    /// Logs the reason readings are absent the first time it is called.
    #[cfg(feature = "gpu-cuda")]
    fn report_absent(&mut self, reason: &str) {
        if !std::mem::replace(&mut self.reported, true) {
            tracing::warn!(target: "piramid::host", %reason, "GPU readings are absent");
        }
    }

    /// Take one reading per device. Always empty, as this build has no driver library.
    #[cfg(not(feature = "gpu-cuda"))]
    pub fn sample(&mut self) -> Vec<GpuReading> {
        Vec::new()
    }
}

impl Default for GpuSampler {
    fn default() -> Self {
        Self::new()
    }
}
