//! Device view state: host readings over time, and handing the terminal to a process monitor.
//! Drawing is in ui, HTTP is in client.

use std::collections::VecDeque;
use std::ffi::OsStr;
use std::net::IpAddr;
use std::path::PathBuf;
use std::time::Instant;

use super::client::HostMetrics;

/// How many refreshes the graphs keep.
const HISTORY: usize = 240;

/// Consecutive points of one reading, as seconds before now against value.
pub type Run = Vec<(f64, f64)>;

/// One refresh of the host readings.
#[derive(Debug, Clone, Copy)]
pub struct Sample {
    /// When the refresh landed.
    pub at: Instant,
    /// What the server reported. Every field is None for a refresh that failed.
    pub host: HostMetrics,
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
    pub fn record(&mut self, at: Instant, host: HostMetrics) {
        if self.samples.len() == HISTORY {
            self.samples.pop_front();
        }
        self.samples.push_back(Sample { at, host });
    }

    /// The readings of the newest refresh.
    pub fn latest(&self) -> Option<&HostMetrics> {
        self.samples.back().map(|sample| &sample.host)
    }

    /// Runs of consecutive readings as points of seconds before now against value.
    ///
    /// A refresh without the reading ends a run, so an absent reading is a gap in the graph.
    pub fn series(&self, now: Instant, read: impl Fn(&HostMetrics) -> Option<f64>) -> Vec<Run> {
        let mut runs: Vec<Run> = Vec::new();
        let mut open = false;
        for sample in &self.samples {
            match read(&sample.host) {
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

    /// The executable to hand the terminal to, or why the console cannot.
    ///
    /// A monitor shows the machine it runs on, so it is refused while the console watches a
    /// server elsewhere. path is the PATH to search.
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
