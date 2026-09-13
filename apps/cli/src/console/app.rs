//! Console state and the key map. Drawing is in ui; processes are in runner.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitStatus;
use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc::UnboundedSender;

use crate::console::client::Client;
use crate::console::collections::{self, Collections, Pending, Reply};
use crate::console::device::{DeviceView, Monitor};
use crate::console::logs::{LogBuffer, LogWriter};
use crate::console::runner::Runner;
use crate::console::settings::Settings;
use crate::console::types::{
    Command, ConfigState, Event, Focus, Health, Kind, LogLine, Mode, Profile, ServiceState, Status,
    Stream, Unit, View,
};
use crate::console::units;

/// A catalog entry plus what the console knows about it right now.
pub struct UnitState {
    /// The catalog entry.
    pub unit: Unit,
    /// Lifecycle.
    pub status: Status,
    /// Captured output.
    pub logs: LogBuffer,
    /// First visible line when not following.
    pub scroll: usize,
    /// Pin the view to the newest line.
    pub follow: bool,
    /// When it was last started here.
    pub started_at: Option<Instant>,
    /// Start again once the current instance has exited.
    restart_pending: bool,
    /// The console asked it to stop, and the exit that follows is not a failure.
    stopping: bool,
}

/// All console state.
pub struct App {
    /// How much of the repo this console can drive.
    pub profile: Profile,
    /// The view keys act on.
    pub view: View,
    /// Collections on a running server.
    pub collections: Collections,
    /// Host readings of the server over time.
    pub device: DeviceView,
    /// A process monitor confirmed as runnable, for the loop to hand the terminal to.
    pub handoff: Option<(Monitor, PathBuf)>,
    /// The resolved configuration. None until it is requested, and again after a reload key.
    pub config: Option<ConfigState>,
    /// First visible line of the config view.
    pub config_scroll: usize,
    settings: Settings,
    runner: Runner,
    log_writer: LogWriter,
    /// Units in sidebar order.
    pub units: Vec<UnitState>,
    /// Index into units.
    pub selected: usize,
    /// Input mode.
    pub mode: Mode,
    /// Pane keys act on.
    pub focus: Focus,
    /// The command or search line being typed.
    pub input: String,
    /// Active log search.
    pub search: String,
    /// Line index of the current search hit.
    pub search_hit: Option<usize>,
    /// Latest probes.
    pub health: Health,
    /// Why the probes are not running, if they are not.
    pub probes_stopped: Option<String>,
    /// The help overlay is open.
    pub help: bool,
    /// One-line notice in the status bar.
    pub notice: Option<String>,
    /// Rows the log pane had at the last draw; drives paging.
    pub log_rows: usize,
    /// Set by the quit key and the quit command.
    pub should_quit: bool,
    /// An action the collections view confirmed, for the loop to dispatch.
    pub pending_action: Option<Pending>,
    /// First half of a two-key chord such as gg.
    pending_key: Option<char>,
}

impl App {
    /// A console over the repo at root.
    pub fn new(
        settings: Settings,
        profile: Profile,
        root: PathBuf,
        tx: &UnboundedSender<Event>,
    ) -> std::io::Result<Self> {
        let log_writer = LogWriter::new(settings.log_dir_under(&root))?;
        let units = units::catalog()
            .into_iter()
            .map(|unit| UnitState::new(unit, settings.log_lines))
            .collect();
        let client = Client::new(
            &settings.base_url,
            std::time::Duration::from_secs(15),
            settings.api_key.clone(),
        )
        .map_err(std::io::Error::other)?;
        Ok(Self {
            profile,
            view: profile
                .views()
                .first()
                .copied()
                .unwrap_or(View::Collections),
            collections: Collections::new(client, settings.refresh),
            device: DeviceView::new(&settings.base_url),
            handoff: None,
            config: None,
            config_scroll: 0,
            runner: Runner::new(root, tx.clone()),
            settings,
            log_writer,
            units,
            selected: 0,
            mode: Mode::Normal,
            focus: Focus::Units,
            input: String::new(),
            search: String::new(),
            search_hit: None,
            health: Health::default(),
            probes_stopped: None,
            help: false,
            notice: None,
            log_rows: 20,
            should_quit: false,
            pending_action: None,
            pending_key: None,
        })
    }

    /// Server location, for the status bar.
    pub fn base_url(&self) -> &str {
        &self.settings.base_url
    }

    /// Website location, for the status bar.
    pub fn web_url(&self) -> &str {
        &self.settings.web_url
    }

    /// The selected unit.
    pub fn current(&self) -> &UnitState {
        let index = self.selected.min(self.units.len().saturating_sub(1));
        &self.units[index]
    }

    fn current_mut(&mut self) -> &mut UnitState {
        let index = self.selected.min(self.units.len().saturating_sub(1));
        &mut self.units[index]
    }

    /// Kills host processes before the terminal is restored.
    pub fn shutdown(&mut self) {
        self.runner.shutdown();
    }

    /// Applies one event.
    pub fn handle(&mut self, event: Event) {
        match event {
            Event::Key(key) => self.key(key),
            Event::Tick | Event::Resize => {}
            Event::Log { unit, line } => self.log(&unit, line),
            Event::Exited { unit, code } => self.exited(&unit, code),
            Event::Services(Ok(states)) => self.services(&states),
            Event::Services(Err(why)) => self.notice = Some(why),
            Event::Health(health) => self.health = *health,
            Event::ProbesStopped(why) => self.probes_stopped = Some(why),
            Event::Snapshot(result) => {
                let (host, gpus, budget, inference) = match result.as_ref() {
                    Ok(snapshot) => (
                        snapshot.metrics.host,
                        snapshot.metrics.gpus.clone(),
                        snapshot.metrics.gpu_budget.clone(),
                        snapshot.metrics.inference.clone(),
                    ),
                    Err(_) => (None, Vec::new(), None, None),
                };
                self.device
                    .record(Instant::now(), host, gpus, budget, inference);
                self.collections.snapshot(*result);
            }
            Event::Acted(Ok(note) | Err(note)) => self.notice = Some(note),
            Event::Config(result) => {
                self.config = Some(match result {
                    Ok(text) => ConfigState::Loaded(text),
                    Err(why) => ConfigState::Failed(why),
                });
            }
            Event::InputLost(why) => {
                self.notice = Some(format!("terminal input ended ({why}); quitting"));
                self.should_quit = true;
            }
        }
    }

    fn log(&mut self, unit: &str, line: LogLine) {
        if let Err(e) = self.log_writer.append(unit, &line) {
            self.notice = Some(format!("write log: {e}"));
        }
        if let Some(state) = self.units.iter_mut().find(|state| state.unit.id == unit) {
            state.logs.push(line);
        }
    }

    fn exited(&mut self, unit: &str, code: Option<i32>) {
        self.runner.forget(unit);
        let (note, restart_id) = {
            let Some(state) = self.units.iter_mut().find(|state| state.unit.id == unit) else {
                return;
            };
            let note = match state.unit.kind {
                // A compose up exit reports the request finishing, not the container stopping.
                // Container state arrives from the compose ps poll.
                Kind::Service { .. } => {
                    if let Some(code) = code.filter(|code| *code != 0) {
                        state.status = Status::Failed(format!("compose exited {code}"));
                    }
                    None
                }
                Kind::Process | Kind::Task => {
                    let stopped = std::mem::take(&mut state.stopping);
                    state.status = match code {
                        Some(code) if !stopped => Status::Exited(code),
                        _ => Status::Stopped,
                    };
                    Some(match code {
                        Some(code) if !stopped => format!("exited with {code}"),
                        _ => "stopped".to_owned(),
                    })
                }
            };
            let restart_id =
                std::mem::take(&mut state.restart_pending).then(|| state.unit.id.clone());
            (note, restart_id)
        };
        if let Some(note) = note {
            self.log(unit, LogLine::now(Stream::Meta, note));
        }
        if let Some(id) = restart_id {
            self.start_by_id(&id);
        }
    }

    fn services(&mut self, states: &HashMap<String, ServiceState>) {
        for state in &mut self.units {
            let Some(service) = state.unit.service() else {
                continue;
            };
            match states.get(service) {
                Some(observed) => state.status = observed.status(),
                // A service asked to start but not yet visible to compose stays Starting.
                None if state.status == Status::Starting => {}
                None => state.status = Status::Stopped,
            }
        }
    }

    fn key(&mut self, key: KeyEvent) {
        self.notice = None;
        if self.help {
            self.help = false;
            return;
        }
        match self.mode {
            Mode::Normal => self.key_normal(key),
            Mode::Command => self.key_line(key, true),
            Mode::Search => self.key_line(key, false),
        }
    }

    fn key_normal(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
                return;
            }
            KeyCode::Char('c') if ctrl => {
                self.should_quit = true;
                return;
            }
            KeyCode::Char('?') => {
                self.help = true;
                return;
            }
            KeyCode::Char(digit @ '1'..='9') => {
                self.select_view(digit);
                return;
            }
            _ => {}
        }
        match self.view {
            View::Units => self.key_units(key),
            View::Collections => match self.collections.key(key) {
                Reply::Nothing => {}
                Reply::Notice(note) => self.notice = Some(note),
                Reply::Dispatch(pending) => {
                    self.notice = Some(format!("{} running", collections::verb(&pending)));
                    self.pending_action = Some(pending);
                }
            },
            View::Config => self.key_config(key),
            View::Device => self.key_device(key),
        }
    }

    fn key_device(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('h') => self.request_handoff(Monitor::Htop),
            KeyCode::Char('n') => self.request_handoff(Monitor::Nvtop),
            KeyCode::Char('R') => self.collections.last_refresh = None,
            KeyCode::Esc => self.notice = None,
            _ => {}
        }
    }

    /// Queues a handoff to monitor, or says why there cannot be one.
    fn request_handoff(&mut self, monitor: Monitor) {
        match self
            .device
            .handoff(monitor, std::env::var_os("PATH").as_deref())
        {
            Ok(program) => self.handoff = Some((monitor, program)),
            Err(why) => self.notice = Some(why),
        }
    }

    /// Records how a handoff to monitor ended, once the terminal is back.
    pub fn handed_back(&mut self, monitor: Monitor, outcome: std::io::Result<ExitStatus>) {
        let program = monitor.program();
        self.notice = match outcome {
            Ok(status) if status.success() => None,
            Ok(status) => Some(match status.code() {
                Some(code) => format!("{program} exited with {code}"),
                None => format!("{program} was killed by a signal"),
            }),
            Err(e) => Some(format!("{program}: {e}")),
        };
    }

    /// Switches to the view a digit names, if this profile offers one there.
    fn select_view(&mut self, digit: char) {
        let index = digit.to_digit(10).unwrap_or(0).saturating_sub(1) as usize;
        match self.profile.views().get(index) {
            Some(view) => {
                self.view = *view;
                self.notice = None;
            }
            None => self.notice = Some(format!("no view {digit}")),
        }
    }

    fn key_config(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => self.config_scroll += 1,
            KeyCode::Char('k') | KeyCode::Up => {
                self.config_scroll = self.config_scroll.saturating_sub(1);
            }
            KeyCode::Char('g') => self.config_scroll = 0,
            KeyCode::Char('R') => self.config = None,
            KeyCode::Esc => self.notice = None,
            _ => {}
        }
    }

    fn key_units(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let chord = self.pending_key.take() == Some('g');
        match key.code {
            KeyCode::Esc => self.notice = None,
            KeyCode::Char('j') | KeyCode::Down => self.down(1),
            KeyCode::Char('k') | KeyCode::Up => self.up(1),
            KeyCode::Char('d') if ctrl => self.down(self.log_rows / 2),
            KeyCode::Char('u') if ctrl => self.up(self.log_rows / 2),
            KeyCode::Char('g') if chord => self.top(),
            KeyCode::Char('g') => self.pending_key = Some('g'),
            KeyCode::Char('G') => self.bottom(),
            KeyCode::Enter | KeyCode::Char('s') => self.toggle_selected(),
            KeyCode::Char('x') => self.stop_selected(),
            KeyCode::Char('r') => self.restart_selected(),
            KeyCode::Char('h') | KeyCode::Left => self.focus = Focus::Units,
            KeyCode::Char('l') | KeyCode::Right => self.focus = Focus::Logs,
            KeyCode::Tab => self.cycle_focus(),
            KeyCode::Char('C') => self.run(Command::Clear),
            KeyCode::Char('o') => self.open_url(),
            KeyCode::Char(':') => {
                self.mode = Mode::Command;
                self.input.clear();
            }
            KeyCode::Char('/') => {
                self.mode = Mode::Search;
                self.focus = Focus::Logs;
                self.input.clear();
            }
            KeyCode::Char('n') => self.search_step(false),
            KeyCode::Char('N') => self.search_step(true),
            _ => {}
        }
    }

    fn key_line(&mut self, key: KeyEvent, command: bool) {
        match key.code {
            KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Enter => {
                let text = std::mem::take(&mut self.input);
                self.mode = Mode::Normal;
                if command {
                    self.run(parse_command(&text));
                } else {
                    self.search = text;
                    self.search_hit = None;
                    self.search_step(false);
                }
            }
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.clear();
            }
            KeyCode::Char(c) => self.input.push(c),
            _ => {}
        }
    }

    fn down(&mut self, n: usize) {
        match self.focus {
            Focus::Units => {
                self.selected = (self.selected + n).min(self.units.len().saturating_sub(1));
                self.on_select();
            }
            Focus::Logs => {
                let rows = self.log_rows;
                let state = self.current_mut();
                let max_top = state.logs.len().saturating_sub(rows);
                state.scroll = (state.scroll + n).min(max_top);
                state.follow = state.scroll >= max_top;
            }
        }
    }

    fn up(&mut self, n: usize) {
        match self.focus {
            Focus::Units => {
                self.selected = self.selected.saturating_sub(n);
                self.on_select();
            }
            Focus::Logs => {
                let rows = self.log_rows;
                let state = self.current_mut();
                if state.follow {
                    state.scroll = state.logs.len().saturating_sub(rows);
                }
                state.scroll = state.scroll.saturating_sub(n);
                state.follow = false;
            }
        }
    }

    fn top(&mut self) {
        match self.focus {
            Focus::Units => {
                self.selected = 0;
                self.on_select();
            }
            Focus::Logs => {
                let state = self.current_mut();
                state.scroll = 0;
                state.follow = false;
            }
        }
    }

    fn bottom(&mut self) {
        match self.focus {
            Focus::Units => {
                self.selected = self.units.len().saturating_sub(1);
                self.on_select();
            }
            Focus::Logs => self.current_mut().follow = true,
        }
    }

    fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Units => Focus::Logs,
            Focus::Logs => Focus::Units,
        };
    }

    /// Follows the container logs of a running service when it is selected, including containers
    /// the console did not start.
    fn on_select(&mut self) {
        self.search_hit = None;
        let state = self.current();
        if state.status != Status::Running {
            return;
        }
        let Some(service) = state.unit.service().map(str::to_owned) else {
            return;
        };
        if let Err(e) = self.runner.follow(&service) {
            self.notice = Some(e.to_string());
        }
    }

    fn open_url(&mut self) {
        let Some(url) = self.current().unit.url.clone() else {
            self.notice = Some("no URL for this unit".into());
            return;
        };
        let url = if url.starts_with("http") {
            url
        } else {
            format!("http://{url}")
        };
        match std::process::Command::new("xdg-open")
            .arg(&url)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            Ok(_) => self.notice = Some(format!("opened {url}")),
            Err(e) => self.notice = Some(format!("xdg-open: {e}")),
        }
    }

    fn search_step(&mut self, backwards: bool) {
        if self.search.is_empty() {
            self.notice = Some("no search; press / first".into());
            return;
        }
        let rows = self.log_rows;
        let needle = self.search.clone();
        let from = self.search_hit;
        let state = self.current_mut();
        let start = from.unwrap_or_else(|| {
            if backwards {
                0
            } else {
                state.logs.len().saturating_sub(1)
            }
        });
        match state.logs.find(&needle, start, backwards) {
            Some(hit) => {
                state.follow = false;
                state.scroll = hit.saturating_sub(rows / 2);
                self.search_hit = Some(hit);
                self.focus = Focus::Logs;
            }
            None => self.notice = Some(format!("no match for {needle:?}")),
        }
    }

    fn toggle_selected(&mut self) {
        if self.current().status.is_active() {
            self.stop_selected();
        } else {
            let id = self.current().unit.id.clone();
            self.start_by_id(&id);
        }
    }

    fn stop_selected(&mut self) {
        let id = self.current().unit.id.clone();
        self.stop_by_id(&id);
    }

    fn restart_selected(&mut self) {
        let id = self.current().unit.id.clone();
        self.run(Command::Restart(id));
    }

    fn index_of(&self, id: &str) -> Option<usize> {
        self.units.iter().position(|state| state.unit.id == id)
    }

    fn start_by_id(&mut self, id: &str) {
        let Some(index) = self.index_of(id) else {
            self.notice = Some(format!("no unit {id:?}"));
            return;
        };
        if self.units[index].status.is_active() {
            self.notice = Some(format!("{id} is already running"));
            return;
        }
        let unit = self.units[index].unit.clone();
        self.selected = index;
        match self.runner.start(&unit) {
            Ok(()) => {
                let state = &mut self.units[index];
                state.status = match unit.kind {
                    Kind::Service { .. } => Status::Starting,
                    Kind::Process | Kind::Task => Status::Running,
                };
                state.started_at = Some(Instant::now());
                state.follow = true;
                self.notice = Some(format!("started {id}"));
            }
            Err(e) => {
                self.units[index].status = Status::Failed(e.to_string());
                self.notice = Some(e.to_string());
            }
        }
    }

    fn stop_by_id(&mut self, id: &str) {
        let Some(index) = self.index_of(id) else {
            self.notice = Some(format!("no unit {id:?}"));
            return;
        };
        let unit = self.units[index].unit.clone();
        if !self.units[index].status.is_active() && !self.runner.owns(id) {
            self.notice = Some(format!("{id} is not running"));
            return;
        }
        match self.runner.stop(&unit) {
            Ok(()) => {
                self.units[index].stopping = true;
                self.notice = Some(format!("stopping {id}"));
            }
            Err(e) => self.notice = Some(e.to_string()),
        }
    }

    /// Executes a parsed command.
    pub fn run(&mut self, command: Command) {
        match command {
            Command::Quit => self.should_quit = true,
            Command::Start(id) => self.start_by_id(&id),
            Command::Stop(id) => self.stop_by_id(&id),
            Command::Restart(id) => self.restart(&id),
            Command::Just(args) => self.run_adhoc(&args),
            Command::Help => self.help = true,
            Command::Clear => self.current_mut().logs.clear(),
            Command::Unknown(text) => {
                self.notice = Some(format!("unknown command {text:?}; try :help"));
            }
        }
    }

    fn restart(&mut self, id: &str) {
        let Some(index) = self.index_of(id) else {
            self.notice = Some(format!("no unit {id:?}"));
            return;
        };
        if self.units[index].status.is_active() || self.runner.owns(id) {
            // The start waits for the exit to arrive.
            self.units[index].restart_pending = true;
            self.stop_by_id(id);
        } else {
            self.start_by_id(id);
        }
    }

    /// Runs any recipe the catalog does not list, as a task of its own.
    fn run_adhoc(&mut self, args: &[String]) {
        if args.is_empty() {
            self.notice = Some("usage: :<recipe> [args]".into());
            return;
        }
        let unit = units::task(args, "ad-hoc");
        let index = match self.index_of(&unit.id) {
            Some(index) => index,
            None => {
                self.units
                    .push(UnitState::new(unit.clone(), self.settings.log_lines));
                self.units.len() - 1
            }
        };
        self.selected = index;
        self.focus = Focus::Logs;
        self.start_by_id(&unit.id);
    }
}

impl UnitState {
    fn new(unit: Unit, log_lines: usize) -> Self {
        Self {
            unit,
            status: Status::Stopped,
            logs: LogBuffer::new(log_lines),
            scroll: 0,
            follow: true,
            started_at: None,
            restart_pending: false,
            stopping: false,
        }
    }
}

/// Parses the text typed on the command line.
///
/// Anything the console does not recognise is passed to just.
pub fn parse_command(text: &str) -> Command {
    let mut words = text.split_whitespace();
    let Some(head) = words.next() else {
        return Command::Unknown(String::new());
    };
    let rest: Vec<String> = words.map(str::to_owned).collect();
    let argument = rest.join(" ");
    match head {
        "q" | "quit" => Command::Quit,
        "start" if !argument.is_empty() => Command::Start(argument),
        "stop" if !argument.is_empty() => Command::Stop(argument),
        "restart" if !argument.is_empty() => Command::Restart(argument),
        "just" => Command::Just(rest),
        "help" | "h" => Command::Help,
        "clear" => Command::Clear,
        _ => Command::Just(std::iter::once(head.to_owned()).chain(rest).collect()),
    }
}
