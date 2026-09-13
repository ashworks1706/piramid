//! The event loop: terminal setup, the tasks that feed it, and the draw cycle.

use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use std::time::Duration;

use crossterm::event::{Event as TermEvent, EventStream, KeyEventKind};
use futures::StreamExt;
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio::task::JoinHandle;

use crate::console::app::App;
use crate::console::client::Client;
use crate::console::collections::Pending;
use crate::console::runner::Runner;
use crate::console::settings::Settings;
use crate::console::types::{ConfigState, Event, Profile};
use crate::console::{health, ui};
use piramid_core::config::Config;

/// How often docker compose ps is re-read.
const SERVICES_INTERVAL: Duration = Duration::from_secs(3);

/// Runs the console over the repo at root.
pub fn run(config: &Config, profile: Profile, root: PathBuf) -> std::io::Result<()> {
    let settings = Settings::from_config(config).map_err(std::io::Error::other)?;
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async move {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut app = App::new(settings.clone(), profile, root.clone(), &tx)?;

        // Read once.
        match app.collections.client.version().await {
            Ok(version) => {
                app.collections.version = match version.git_commit {
                    Some(commit) => format!("v{} {commit}", version.version),
                    None => format!("v{}", version.version),
                };
            }
            Err(e) => app.notice = Some(format!("server version unknown: {e}")),
        }

        tokio::spawn(health::poll(
            health::Targets::from_settings(&settings),
            profile == Profile::Developer,
            tx.clone(),
        ));
        // Compose is polled only inside a checkout.
        if profile == Profile::Developer {
            tokio::spawn(services(root, tx.clone()));
        }
        tokio::spawn(ticker(tx.clone()));
        let mut input = tokio::spawn(keys(tx.clone()));

        // The terminal is restored before a panic message is printed.
        let original_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            ratatui::restore();
            original_hook(info);
        }));
        let mut terminal = ratatui::init();
        let outcome = drive(&mut terminal, &mut app, &mut rx, &tx, &mut input).await;
        app.shutdown();
        ratatui::restore();
        outcome
    })
}

async fn drive(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    rx: &mut mpsc::UnboundedReceiver<Event>,
    tx: &UnboundedSender<Event>,
    input: &mut JoinHandle<()>,
) -> std::io::Result<()> {
    refresh(app, tx);
    fetch_config(app, tx);
    terminal.draw(|frame| ui::draw(frame, app))?;
    while let Some(event) = rx.recv().await {
        app.handle(event);
        // Everything else already queued is drained into one redraw.
        while let Ok(more) = rx.try_recv() {
            app.handle(more);
        }
        if app.should_quit {
            break;
        }
        if let Some(pending) = app.pending_action.take() {
            act(app.collections.client.clone(), tx.clone(), pending);
        }
        if let Some((monitor, program)) = app.handoff.take() {
            let outcome = hand_over(terminal, &program, input, tx).await;
            app.handed_back(monitor, outcome);
        }
        if app.collections.refresh_due() {
            refresh(app, tx);
        }
        if app.config.is_none() {
            fetch_config(app, tx);
        }
        terminal.draw(|frame| ui::draw(frame, app))?;
    }
    Ok(())
}

/// Runs program in the foreground with the terminal it needs, then takes the terminal back.
///
/// The key reader is stopped while program runs.
async fn hand_over(
    terminal: &mut ratatui::DefaultTerminal,
    program: &Path,
    input: &mut JoinHandle<()>,
    tx: &UnboundedSender<Event>,
) -> std::io::Result<ExitStatus> {
    input.abort();
    let _ = (&mut *input).await;
    ratatui::restore();
    let status = tokio::process::Command::new(program).status().await;
    let resumed = resume_terminal(terminal);
    *input = tokio::spawn(keys(tx.clone()));
    resumed?;
    status
}

/// Puts the terminal back into the raw alternate screen the console draws on.
fn resume_terminal(terminal: &mut ratatui::DefaultTerminal) -> std::io::Result<()> {
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
    terminal.clear()
}

async fn services(root: PathBuf, tx: UnboundedSender<Event>) {
    loop {
        let states = Runner::service_states(&root).await;
        if tx.send(Event::Services(states)).is_err() {
            return;
        }
        tokio::time::sleep(SERVICES_INTERVAL).await;
    }
}

/// Redraws on a timer, advancing the elapsed-time counters while nothing else happens.
async fn ticker(tx: UnboundedSender<Event>) {
    let mut interval = tokio::time::interval(Duration::from_millis(500));
    loop {
        interval.tick().await;
        if tx.send(Event::Tick).is_err() {
            return;
        }
    }
}

async fn keys(tx: UnboundedSender<Event>) {
    let mut stream = EventStream::new();
    while let Some(event) = stream.next().await {
        let mapped = match event {
            Ok(TermEvent::Key(key)) if key.kind == KeyEventKind::Press => Event::Key(key),
            Ok(TermEvent::Resize(_, _)) => Event::Resize,
            Ok(_) => continue,
            // A lost keyboard is reported and shuts the console down.
            Err(e) => Event::InputLost(e.to_string()),
        };
        if tx.send(mapped).is_err() {
            return;
        }
    }
}

/// Starts a collections refresh.
fn refresh(app: &mut App, tx: &UnboundedSender<Event>) {
    app.collections.refreshing = true;
    let client = app.collections.client.clone();
    let tx = tx.clone();
    tokio::spawn(async move {
        let snapshot = client.snapshot().await;
        let _ = tx.send(Event::Snapshot(Box::new(snapshot)));
    });
}

/// Reads the configuration the server resolved.
fn fetch_config(app: &mut App, tx: &UnboundedSender<Event>) {
    app.config = Some(ConfigState::Loading);
    let client = app.collections.client.clone();
    let tx = tx.clone();
    tokio::spawn(async move {
        let rendered = client.config().await.map_err(|e| e.to_string());
        let _ = tx.send(Event::Config(rendered));
    });
}

/// Runs a confirmed action and reports what it did.
fn act(client: Client, tx: UnboundedSender<Event>, pending: Pending) {
    let name = crate::console::collections::verb(&pending);
    tokio::spawn(async move {
        let outcome = match &pending {
            Pending::Compact(collection) => match client.compact(collection).await {
                Ok(compacted) => Ok(crate::console::collections::compacted_line(
                    collection, &compacted,
                )),
                Err(e) => Err(format!("{name} failed: {e}")),
            },
        };
        let _ = tx.send(Event::Acted(outcome));
    });
}
