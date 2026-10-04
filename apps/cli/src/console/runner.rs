//! Starts, stops and streams the output of units.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc::UnboundedSender;

use crate::console::types::{Event, LogLine, RunnerError, Stream, Unit};

/// Owns the children the console started.
pub struct Runner {
    root: PathBuf,
    tx: UnboundedSender<Event>,
    /// Process-group ids of host processes and tasks, by unit id.
    groups: HashMap<String, u32>,
}

impl Runner {
    /// A runner working from the repo root.
    pub fn new(root: PathBuf, tx: UnboundedSender<Event>) -> Self {
        Self {
            root,
            tx,
            groups: HashMap::new(),
        }
    }

    /// Starts a unit in its own process group.
    pub fn start(&mut self, unit: &Unit) -> Result<(), RunnerError> {
        let mut cmd = Command::new("setsid");
        cmd.arg("just").args(&unit.args);
        self.spawn_streaming(&unit.id, cmd)
    }

    /// Stops a unit by sending SIGTERM to its process group.
    pub fn stop(&mut self, unit: &Unit) -> Result<(), RunnerError> {
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

    /// Whether a host process or task started here is still tracked.
    pub fn owns(&self, unit_id: &str) -> bool {
        self.groups.contains_key(unit_id)
    }

    /// Drops the record of a process group after its child exited.
    pub fn forget(&mut self, unit_id: &str) {
        self.groups.remove(unit_id);
    }

    /// Kills every host process and task.
    pub fn shutdown(&mut self) {
        for pgid in self.groups.values() {
            let _ = std::process::Command::new("kill")
                .args(["-TERM", "--", &format!("-{pgid}")])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
        self.groups.clear();
    }

    /// Spawns, streams both outputs as log lines and reports the exit.
    fn spawn_streaming(&mut self, unit_id: &str, cmd: Command) -> Result<(), RunnerError> {
        let mut child = self.spawn_piped(unit_id, cmd)?;
        let Some(pid) = child.id() else {
            let _ = child.start_kill();
            return Err(RunnerError::NotTracked {
                unit: unit_id.to_owned(),
            });
        };
        self.groups.insert(unit_id.to_owned(), pid);
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

    fn spawn_piped(&self, unit_id: &str, mut cmd: Command) -> Result<Child, RunnerError> {
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
        self.note(unit_id, format!("$ {line}"));
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
