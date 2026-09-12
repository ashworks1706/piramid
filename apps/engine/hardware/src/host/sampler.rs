//! Takes host readings from the operating system.

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

use crate::host::reading::HostReading;

/// Reads host processor and memory use.
///
/// Processor use is the change between two samples, so the first sample carries no processor
/// fields. Samples taken closer together than [HostSampler::MINIMUM_INTERVAL] measure too short a
/// span to be accurate.
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
        let cpu_percent = (primed && cpus > 0).then(|| self.system.global_cpu_usage());

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
                        (primed && cpus > 0).then(|| share_of_host(process.cpu_usage(), cpus)),
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
fn share_of_host(percent_of_one_cpu: f32, cpus: usize) -> f32 {
    let cpus = u16::try_from(cpus).unwrap_or(u16::MAX);
    percent_of_one_cpu / f32::from(cpus)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_sample_carries_no_processor_reading() {
        let mut sampler = HostSampler::new();
        let reading = sampler.sample();
        assert_eq!(reading.cpu_percent, None);
        assert_eq!(reading.process_cpu_percent, None);
    }

    #[test]
    fn a_second_sample_reads_processor_use_on_a_supported_system() {
        let mut sampler = HostSampler::new();
        sampler.sample();
        std::thread::sleep(HostSampler::MINIMUM_INTERVAL);
        let reading = sampler.sample();
        if !sysinfo::IS_SUPPORTED_SYSTEM {
            assert_eq!(reading, HostReading::default());
            return;
        }
        let cpu = reading.cpu_percent.unwrap_or(-1.0);
        assert!((0.0..=100.0).contains(&cpu), "cpu_percent {cpu}");
        let process = reading.process_cpu_percent.unwrap_or(-1.0);
        assert!(
            (0.0..=100.0).contains(&process),
            "process_cpu_percent {process}"
        );
    }

    #[test]
    fn memory_is_read_from_the_first_sample_on_a_supported_system() {
        let reading = HostSampler::new().sample();
        if !sysinfo::IS_SUPPORTED_SYSTEM {
            assert_eq!(reading, HostReading::default());
            return;
        }
        let total = reading.memory_total_bytes.unwrap_or(0);
        let used = reading.memory_used_bytes.unwrap_or(u64::MAX);
        assert!(total > 0);
        assert!(used <= total);
        assert!(reading.process_resident_bytes.is_some_and(|rss| rss > 0));
    }

    #[test]
    fn process_use_is_scaled_to_every_cpu_of_the_host() {
        assert!((share_of_host(200.0, 8) - 25.0).abs() < f32::EPSILON);
        assert!((share_of_host(50.0, 1) - 50.0).abs() < f32::EPSILON);
    }
}
