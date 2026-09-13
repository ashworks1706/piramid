//! Device view state: host, GPU, memory budget and generation readings, and handing off a monitor.

use std::collections::VecDeque;
use std::ffi::OsStr;
use std::net::IpAddr;
use std::path::PathBuf;
use std::time::Instant;

use super::client::{GpuBudget, GpuMetrics, HostMetrics, InferenceMetrics};

/// How many refreshes the graphs keep.
const HISTORY: usize = 240;

/// Consecutive points of one reading, as seconds before now against value.
pub type Run = Vec<(f64, f64)>;

/// One refresh of the machine readings.
#[derive(Debug, Clone)]
pub struct Sample {
    /// When the refresh landed.
    pub at: Instant,
    /// What the server reported. None for a refresh that failed.
    pub host: Option<HostMetrics>,
    /// One entry per GPU the server measured. Empty for a refresh that failed or measured none.
    pub gpus: Vec<GpuMetrics>,
    /// The device memory budget. None for a refresh that failed or a server with no GPU open.
    pub budget: Option<GpuBudget>,
    /// Generation readings. None for a refresh that failed or a server with no model loaded.
    pub inference: Option<InferenceMetrics>,
}

/// A process monitor the terminal can be handed to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Monitor {
    /// htop, for processors, memory and processes.
    Htop,
    /// nvtop, for GPUs.
    Nvtop,
}

impl Monitor {
    /// The executable name looked up on PATH.
    pub fn program(self) -> &'static str {
        match self {
            Self::Htop => "htop",
            Self::Nvtop => "nvtop",
        }
    }
}

/// Device view state.
#[derive(Debug)]
pub struct DeviceView {
    /// Refreshes in arrival order, oldest first.
    pub samples: VecDeque<Sample>,
    base_url: String,
    local: bool,
}

impl DeviceView {
    /// A device view of the server at base_url.
    pub fn new(base_url: &str) -> Self {
        Self {
            samples: VecDeque::with_capacity(HISTORY),
            base_url: base_url.to_owned(),
            local: is_loopback(base_url),
        }
    }

    /// Whether the server watched is on this machine.
    pub fn local(&self) -> bool {
        self.local
    }

    /// Appends one refresh, dropping the oldest once the history is full.
    pub fn record(
        &mut self,
        at: Instant,
        host: Option<HostMetrics>,
        gpus: Vec<GpuMetrics>,
        budget: Option<GpuBudget>,
        inference: Option<InferenceMetrics>,
    ) {
        if self.samples.len() == HISTORY {
            self.samples.pop_front();
        }
        self.samples.push_back(Sample {
            at,
            host,
            gpus,
            budget,
            inference,
        });
    }

    /// The readings of the newest refresh, if it has any.
    pub fn latest(&self) -> Option<&HostMetrics> {
        self.samples.back().and_then(|sample| sample.host.as_ref())
    }

    /// The readings of the GPU at index in the newest refresh, if it has any.
    pub fn latest_gpu(&self, index: u32) -> Option<&GpuMetrics> {
        self.samples
            .back()
            .and_then(|sample| sample.gpus.iter().find(|gpu| gpu.index == index))
    }

    /// The device memory budget of the newest refresh, if it has one.
    pub fn latest_budget(&self) -> Option<&GpuBudget> {
        self.samples
            .back()
            .and_then(|sample| sample.budget.as_ref())
    }

    /// The generation readings of the newest refresh, if it has any.
    pub fn latest_inference(&self) -> Option<&InferenceMetrics> {
        self.samples
            .back()
            .and_then(|sample| sample.inference.as_ref())
    }

    /// The index of every GPU any refresh in the history reported, in ascending order.
    pub fn gpu_indices(&self) -> Vec<u32> {
        let mut indices: Vec<u32> = self
            .samples
            .iter()
            .flat_map(|sample| sample.gpus.iter().map(|gpu| gpu.index))
            .collect();
        indices.sort_unstable();
        indices.dedup();
        indices
    }

    /// Runs of consecutive host readings as points of seconds before now against value.
    pub fn series(&self, now: Instant, read: impl Fn(&HostMetrics) -> Option<f64>) -> Vec<Run> {
        self.runs(now, |sample| sample.host.as_ref().and_then(&read))
    }

    /// Runs of consecutive readings of the GPU at index, as points of seconds before now vs value.
    pub fn gpu_series(
        &self,
        now: Instant,
        index: u32,
        read: impl Fn(&GpuMetrics) -> Option<f64>,
    ) -> Vec<Run> {
        self.runs(now, |sample| {
            sample
                .gpus
                .iter()
                .find(|gpu| gpu.index == index)
                .and_then(&read)
        })
    }

    /// Runs of consecutive generation readings as points of seconds before now against value.
    pub fn inference_series(
        &self,
        now: Instant,
        read: impl Fn(&InferenceMetrics) -> Option<f64>,
    ) -> Vec<Run> {
        self.runs(now, |sample| sample.inference.as_ref().and_then(&read))
    }

    /// Runs of consecutive values read from each refresh, split wherever read gives None.
    fn runs(&self, now: Instant, read: impl Fn(&Sample) -> Option<f64>) -> Vec<Run> {
        let mut runs: Vec<Run> = Vec::new();
        let mut open = false;
        for sample in &self.samples {
            match read(sample) {
                Some(value) => {
                    let x = -now.saturating_duration_since(sample.at).as_secs_f64();
                    match runs.last_mut().filter(|_| open) {
                        Some(run) => run.push((x, value)),
                        None => runs.push(vec![(x, value)]),
                    }
                    open = true;
                }
                None => open = false,
            }
        }
        runs
    }

    /// The executable to hand the terminal to, or why the console cannot, searching path.
    pub fn handoff(&self, monitor: Monitor, path: Option<&OsStr>) -> Result<PathBuf, String> {
        let program = monitor.program();
        if !self.local {
            return Err(format!(
                "{program} shows this machine, and the console watches {}",
                self.base_url
            ));
        }
        find_program(program, path)
            .ok_or_else(|| format!("{program} is not installed: not found on PATH"))
    }
}

/// Whether the host of base_url is a loopback name or address.
pub fn is_loopback(base_url: &str) -> bool {
    let rest = base_url
        .split_once("://")
        .map_or(base_url, |(_, rest)| rest);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host_port = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    let host = match host_port.strip_prefix('[') {
        Some(bracketed) => bracketed.split(']').next().unwrap_or_default(),
        None => host_port.split(':').next().unwrap_or_default(),
    };
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

/// The first executable file named program in the directories of path.
pub fn find_program(program: &str, path: Option<&OsStr>) -> Option<PathBuf> {
    std::env::split_paths(path?)
        .map(|dir| dir.join(program))
        .find(|candidate| is_executable(candidate))
}

#[cfg(unix)]
fn is_executable(candidate: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(candidate)
        .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(candidate: &std::path::Path) -> bool {
    candidate.is_file()
}
