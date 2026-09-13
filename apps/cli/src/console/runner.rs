//! Starts, stops and streams the output of units.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc::UnboundedSender;

use crate::console::types::{Event, Kind, LogLine, RunnerError, ServiceState, Stream, Unit};

const COMPOSE_FILE: &str = "deploy/compose.yml";

/// Every compose profile in deploy/compose.yml, passed to ps and logs.
const PROFILES: [&str; 1] = ["ollama"];

/// Owns the children the console started.
pub struct Runner {
    root: PathBuf,
    tx: UnboundedSender<Event>,
    /// Process-group ids of host processes and tasks, by unit id.
    groups: HashMap<String, u32>,
    /// Children following container logs, by service name.
    followers: HashMap<String, Child>,
}

impl Runner {
    /// A runner working from the repo root.
    pub fn new(root: PathBuf, tx: UnboundedSender<Event>) -> Self {
        Self {
            root,
            tx,
            groups: HashMap::new(),
            followers: HashMap::new(),
        }
    }

    /// Starts a unit. Services come up detached and are then followed.
    pub fn start(&mut self, unit: &Unit) -> Result<(), RunnerError> {
        match &unit.kind {
            Kind::Service { service, profile } => {
                let mut cmd = self.compose(profile.as_deref());
                cmd.args(["up", "-d", service]);
                self.spawn_streaming(&unit.id, cmd, true)?;
                self.follow(service)
            }
            Kind::Process | Kind::Task => {
                let mut cmd = Command::new("setsid");
                cmd.arg("just").args(&unit.args);
                self.spawn_streaming(&unit.id, cmd, true)
            }
        }
    }

    /// Stops a unit. Host trees get SIGTERM, services get a compose stop.
    pub fn stop(&mut self, unit: &Unit) -> Result<(), RunnerError> {
        match &unit.kind {
            Kind::Service { service, profile } => {
                self.unfollow(service);
                let mut cmd = self.compose(profile.as_deref());
                cmd.args(["stop", service]);
                self.spawn_streaming(&unit.id, cmd, false)
            }
            Kind::Process | Kind::Task => {
                let pgid = self
                    .groups
                    .remove(&unit.id)
                    .ok_or_else(|| RunnerError::NotTracked {
                        unit: unit.id.clone(),
                    })?;
                self.note(&unit.id, format!("stopping process group {pgid}"));
                let mut kill = Command::new("kill");
                kill.args(["-TERM", "--", &format!("-{pgid}")]);
                kill.stdout(Stdio::null()).stderr(Stdio::null());
                kill.spawn().map_err(|source| RunnerError::Spawn {
                    cmd: format!("kill -TERM -- -{pgid}"),
                    source,
                })?;
                Ok(())
            }
        }
    }

    /// Whether a host process or task started here is still tracked.
    pub fn owns(&self, unit_id: &str) -> bool {
        self.groups.contains_key(unit_id)
    }

    /// Drops the record of a process group after its child exited.
    pub fn forget(&mut self, unit_id: &str) {
        self.groups.remove(unit_id);
    }

    /// Begins streaming the container logs of a service, if not already following.
    pub fn follow(&mut self, service: &str) -> Result<(), RunnerError> {
        if self.followers.contains_key(service) {
            return Ok(());
        }
        let mut cmd = self.compose_all_profiles();
        cmd.args(["logs", "-f", "--tail", "200", "--no-color", service]);
        let child = self.spawn_piped(service, cmd, false)?;
        self.followers.insert(service.to_owned(), child);
        Ok(())
    }

    /// Stops following the logs of a service.
    pub fn unfollow(&mut self, service: &str) {
        if let Some(mut child) = self.followers.remove(service) {
            let _ = child.start_kill();
        }
    }

    /// Kills every host process and follower. Compose services are left running.
    pub fn shutdown(&mut self) {
        let services: Vec<String> = self.followers.keys().cloned().collect();
        for service in services {
            self.unfollow(&service);
        }
        for pgid in self.groups.values() {
            let _ = std::process::Command::new("kill")
                .args(["-TERM", "--", &format!("-{pgid}")])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
        self.groups.clear();
    }

    /// One docker compose ps snapshot, keyed by service.
    pub async fn service_states(root: &Path) -> Result<HashMap<String, ServiceState>, String> {
        let output = Command::new("docker")
            .args(["compose", "-f", COMPOSE_FILE])
            .args(profile_args())
            .args(["ps", "-a", "--format", "json"])
            .current_dir(root)
            .output()
            .await
            .map_err(|e| format!("docker compose ps: {e}"))?;
        if !output.status.success() {
            return Err(format!(
                "docker compose ps: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        parse_ps(&String::from_utf8_lossy(&output.stdout))
    }

    fn compose(&self, profile: Option<&str>) -> Command {
        let mut cmd = Command::new("docker");
        cmd.args(["compose", "-f", COMPOSE_FILE]);
        if let Some(profile) = profile {
            cmd.args(["--profile", profile]);
        }
        cmd.current_dir(&self.root);
        cmd
    }

    fn compose_all_profiles(&self) -> Command {
        let mut cmd = Command::new("docker");
        cmd.args(["compose", "-f", COMPOSE_FILE]);
        cmd.args(profile_args());
        cmd.current_dir(&self.root);
        cmd
    }

    /// Spawns, streams both outputs as log lines and reports the exit.
    fn spawn_streaming(
        &mut self,
        unit_id: &str,
        cmd: Command,
        track: bool,
    ) -> Result<(), RunnerError> {
        let mut child = self.spawn_piped(unit_id, cmd, true)?;
        if track {
            let Some(pid) = child.id() else {
                let _ = child.start_kill();
                return Err(RunnerError::NotTracked {
                    unit: unit_id.to_owned(),
                });
            };
            self.groups.insert(unit_id.to_owned(), pid);
        }
        let tx = self.tx.clone();
        let id = unit_id.to_owned();
        tokio::spawn(async move {
            let code = match child.wait().await {
                Ok(status) => status.code(),
                Err(e) => {
                    let _ = tx.send(Event::Log {
                        unit: id.clone(),
                        line: LogLine::now(Stream::Meta, format!("wait failed: {e}")),
                    });
                    None
                }
            };
            let _ = tx.send(Event::Exited { unit: id, code });
        });
        Ok(())
    }

    fn spawn_piped(
        &self,
        unit_id: &str,
        mut cmd: Command,
        announce: bool,
    ) -> Result<Child, RunnerError> {
        let line = describe(cmd.as_std());
        cmd.current_dir(&self.root)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(false)
            .env("CARGO_TERM_COLOR", "never")
            .env("NO_COLOR", "1");
        let mut child = cmd.spawn().map_err(|source| RunnerError::Spawn {
            cmd: line.clone(),
            source,
        })?;
        if announce {
            self.note(unit_id, format!("$ {line}"));
        }
        if let Some(out) = child.stdout.take() {
            pump(self.tx.clone(), unit_id.to_owned(), Stream::Out, out);
        }
        if let Some(err) = child.stderr.take() {
            pump(self.tx.clone(), unit_id.to_owned(), Stream::Err, err);
        }
        Ok(child)
    }

    fn note(&self, unit_id: &str, text: String) {
        let _ = self.tx.send(Event::Log {
            unit: unit_id.to_owned(),
            line: LogLine::now(Stream::Meta, text),
        });
    }
}

fn profile_args() -> Vec<String> {
    PROFILES
        .iter()
        .flat_map(|profile| ["--profile".to_owned(), (*profile).to_owned()])
        .collect()
}

fn describe(cmd: &std::process::Command) -> String {
    let mut parts = vec![cmd.get_program().to_string_lossy().into_owned()];
    parts.extend(cmd.get_args().map(|arg| arg.to_string_lossy().into_owned()));
    parts.join(" ")
}

fn pump<R>(tx: UnboundedSender<Event>, unit: String, stream: Stream, reader: R)
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        loop {
            let text = match lines.next_line().await {
                Ok(Some(text)) => text,
                Ok(None) => break,
                // A capture failure is reported into the pane.
                Err(e) => {
                    let _ = tx.send(Event::Log {
                        unit: unit.clone(),
                        line: LogLine::now(Stream::Meta, format!("log capture ended: {e}")),
                    });
                    break;
                }
            };
            let text = sanitize_line(&text);
            if tx
                .send(Event::Log {
                    unit: unit.clone(),
                    line: LogLine::now(stream, text),
                })
                .is_err()
            {
                break;
            }
        }
    });
}

/// Strips escape sequences from child output, leaving plain text.
pub fn sanitize_line(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for tail in chars.by_ref() {
                    if tail.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        // Tab is the only control character a pane keeps.
        if c.is_control() && c != '\t' {
            continue;
        }
        out.push(c);
    }
    out
}

/// Parses the JSON output of docker compose ps: an array of objects, or one object per line.
pub fn parse_ps(raw: &str) -> Result<HashMap<String, ServiceState>, String> {
    #[derive(serde::Deserialize)]
    struct Row {
        #[serde(rename = "Service")]
        service: String,
        #[serde(rename = "State")]
        state: String,
        #[serde(rename = "Health")]
        health: String,
        #[serde(rename = "ExitCode")]
        exit_code: i32,
    }
    let rows: Vec<Row> = if raw.trim().is_empty() {
        Vec::new()
    } else if let Ok(rows) = serde_json::from_str::<Vec<Row>>(raw) {
        rows
    } else {
        raw.lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                serde_json::from_str::<Row>(line).map_err(|e| format!("compose ps row: {e}"))
            })
            .collect::<Result<_, _>>()?
    };
    Ok(rows
        .into_iter()
        .map(|row| {
            (
                row.service,
                ServiceState {
                    state: row.state,
                    health: row.health,
                    exit_code: row.exit_code,
                },
            )
        })
        .collect())
}
