//! What the console passes between its modules: units, statuses, log lines, modes, events.

use std::collections::HashMap;

use chrono::{DateTime, Local};

/// Sidebar section a unit belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Group {
    /// Long-running host processes: the server, the website.
    Apps,
    /// Docker compose services.
    Containers,
    /// Recipes that run to completion.
    Tasks,
    /// Compose stacks and images, the recipes a deploy host uses.
    Deploy,
}

impl Group {
    /// Sidebar heading.
    pub fn title(self) -> &'static str {
        match self {
            Self::Apps => "apps",
            Self::Containers => "containers",
            Self::Tasks => "tasks",
            Self::Deploy => "deploy",
        }
    }

    /// Display order.
    pub const ALL: [Self; 4] = [Self::Apps, Self::Containers, Self::Tasks, Self::Deploy];
}

/// How a unit is run and stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// A docker compose service. The profile field gates the optional ones.
    Service {
        /// Compose service name.
        service: String,
        /// Compose profile, if the service needs one.
        profile: Option<String>,
    },
    /// A long-running host process started through a just recipe.
    Process,
    /// A just recipe that runs to completion.
    Task,
}

/// Something the console can start, stop and watch.
#[derive(Debug, Clone)]
pub struct Unit {
    /// Stable name shown in the sidebar and used in commands.
    pub id: String,
    /// Sidebar section.
    pub group: Group,
    /// How to run it.
    pub kind: Kind,
    /// Arguments passed to just, for processes and tasks.
    pub args: Vec<String>,
    /// One-line description.
    pub hint: String,
    /// Where it listens, if it does.
    pub url: Option<String>,
}

impl Unit {
    /// Compose service name, for services.
    pub fn service(&self) -> Option<&str> {
        match &self.kind {
            Kind::Service { service, .. } => Some(service),
            _ => None,
        }
    }
}

/// Where a unit is in its lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// Not running.
    Stopped,
    /// Start requested, with no confirmation yet.
    Starting,
    /// Up.
    Running,
    /// Ran and exited with this code.
    Exited(i32),
    /// Could not be started or watched.
    Failed(String),
}

impl Status {
    /// Whether stopping it makes sense.
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Starting | Self::Running)
    }

    /// Single-character marker for the sidebar.
    pub fn glyph(&self) -> &'static str {
        match self {
            Self::Stopped => "○",
            Self::Starting => "◐",
            Self::Running => "●",
            Self::Exited(0) => "✓",
            Self::Exited(_) => "✗",
            Self::Failed(_) => "!",
        }
    }

    /// Short label for the log pane title.
    pub fn label(&self) -> String {
        match self {
            Self::Stopped => "stopped".into(),
            Self::Starting => "starting".into(),
            Self::Running => "running".into(),
            Self::Exited(code) => format!("exit {code}"),
            Self::Failed(why) => format!("failed: {why}"),
        }
    }
}

/// Which output a log line came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stream {
    /// Child stdout.
    Out,
    /// Child stderr.
    Err,
    /// A note the console wrote about the unit.
    Meta,
}

/// One captured line.
#[derive(Debug, Clone)]
pub struct LogLine {
    /// When it was captured.
    pub at: DateTime<Local>,
    /// Source.
    pub stream: Stream,
    /// Content, without the trailing newline.
    pub text: String,
}

impl LogLine {
    /// A line captured now.
    pub fn now(stream: Stream, text: impl Into<String>) -> Self {
        Self {
            at: Local::now(),
            stream,
            text: text.into(),
        }
    }
}

/// How much of the repo the console can drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    /// Started inside a checkout. Every view is available, including the repo units.
    Developer,
    /// Started from an installed binary. Views that drive just recipes and compose are absent.
    Production,
}

impl Profile {
    /// The views this profile offers, in tab order.
    pub fn views(self) -> &'static [View] {
        match self {
            Self::Developer => &[View::Units, View::Collections, View::Config, View::Device],
            Self::Production => &[View::Collections, View::Config, View::Device],
        }
    }
}

/// One screen of the console.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// Repo units: the server, the website, containers, recipes.
    Units,
    /// Collections on a running server, with dimension, memory, latency and durability.
    Collections,
    /// The configuration as the server resolved it.
    Config,
    /// Host, GPU, device memory budget and generation readings of the server over time.
    Device,
}

impl View {
    /// Tab label.
    pub fn title(self) -> &'static str {
        match self {
            Self::Units => "units",
            Self::Collections => "collections",
            Self::Config => "config",
            Self::Device => "device",
        }
    }
}

/// Input mode, vim-style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Keys navigate and act.
    Normal,
    /// Typing a command.
    Command,
    /// Typing a search over the logs of the selected unit.
    Search,
}

/// Which pane keys act on in normal mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    /// The unit list.
    Units,
    /// The log pane.
    Logs,
}

/// One row of docker compose ps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceState {
    /// One of running, exited, created, restarting, paused, dead.
    pub state: String,
    /// One of healthy, unhealthy, starting, or empty without a healthcheck.
    pub health: String,
    /// Last exit code.
    pub exit_code: i32,
}

impl ServiceState {
    /// Maps a compose row onto a console status.
    pub fn status(&self) -> Status {
        match (self.state.as_str(), self.health.as_str()) {
            ("running", "unhealthy") => Status::Failed("unhealthy".into()),
            ("running", "starting") | ("created" | "restarting", _) => Status::Starting,
            ("running", _) => Status::Running,
            ("exited", _) if self.exit_code == 0 => Status::Stopped,
            ("exited" | "dead", _) => Status::Exited(self.exit_code),
            (other, _) => Status::Failed(other.to_owned()),
        }
    }
}

/// Result of one probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Probe {
    /// Not checked yet.
    Unknown,
    /// Reachable and healthy.
    Up,
    /// Reachable but reporting a problem, with its message.
    Degraded(String),
    /// Not reachable, with the reason the request failed.
    Down(String),
}

/// Where the configuration view is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigState {
    /// A request for it is in flight.
    Loading,
    /// The configuration rendered as YAML.
    Loaded(String),
    /// Why it could not be read.
    Failed(String),
}

/// Liveness of what the console watches.
#[derive(Debug, Clone)]
pub struct Health {
    /// The health endpoint: the server process is up.
    pub live: Probe,
    /// The readiness endpoint: every collection on disk opens and passes its integrity check.
    pub ready: Probe,
    /// The website dev or preview server.
    pub web: Probe,
}

impl Default for Health {
    fn default() -> Self {
        Self {
            live: Probe::Unknown,
            ready: Probe::Unknown,
            web: Probe::Unknown,
        }
    }
}

/// Everything that can wake the console.
#[derive(Debug)]
pub enum Event {
    /// A key press.
    Key(crossterm::event::KeyEvent),
    /// Redraw timer.
    Tick,
    /// Terminal resized.
    Resize,
    /// A unit produced a line.
    Log {
        /// Unit id.
        unit: String,
        /// The line.
        line: LogLine,
    },
    /// A process or task ended.
    Exited {
        /// Unit id.
        unit: String,
        /// Exit code, if the process was not killed by a signal.
        code: Option<i32>,
    },
    /// Fresh compose service states by service name, or why docker compose ps failed.
    Services(Result<HashMap<String, ServiceState>, String>),
    /// Fresh probes.
    Health(Box<Health>),
    /// The probes cannot run, with the reason.
    ProbesStopped(String),
    /// A collections refresh finished.
    Snapshot(Box<Result<super::client::Snapshot, super::client::ClientError>>),
    /// A compaction finished, with the line to show for it.
    Acted(Result<String, String>),
    /// The configuration as the server resolved it, or why it could not be read.
    Config(Result<String, String>),
    /// The terminal stopped delivering input, leaving the console undrivable.
    InputLost(String),
}

/// Runner failures.
#[derive(Debug, thiserror::Error)]
pub enum RunnerError {
    /// The child could not be spawned.
    #[error("spawn `{cmd}`: {source}")]
    Spawn {
        /// Command line attempted.
        cmd: String,
        /// OS error.
        source: std::io::Error,
    },
    /// A stop was asked of a unit with no process group this console started.
    #[error("{unit} has no process group this console started")]
    NotTracked {
        /// Unit id.
        unit: String,
    },
}

/// A parsed command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Leave the console, stopping host processes.
    Quit,
    /// Start a unit by id.
    Start(String),
    /// Stop a unit by id.
    Stop(String),
    /// Stop then start a unit by id.
    Restart(String),
    /// Run an arbitrary just recipe as an ad-hoc task.
    Just(Vec<String>),
    /// Show the key map.
    Help,
    /// Clear the logs of the selected unit.
    Clear,
    /// Not understood, with the reason.
    Unknown(String),
}
