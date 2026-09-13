//! Collections view state and its key map. Drawing is in ui, HTTP is in client.

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent};

use super::client::{
    Client, ClientError, CollectionHealth, CollectionInfo, CollectionMetrics, Snapshot, WalStats,
};

/// How many samples the latency sparkline keeps per collection.
const HISTORY: usize = 240;

/// One collection, as every endpoint together describes it.
#[derive(Debug, Clone, Default)]
pub struct Row {
    /// Collection name.
    pub name: String,
    /// Counters from the metrics response, absent for a collection that has never been opened.
    pub metrics: Option<CollectionMetrics>,
    /// Durability, from the same response.
    pub wal: Option<WalStats>,
    /// Summary from the collection list, absent for a collection that is not open.
    pub info: Option<CollectionInfo>,
    /// What readiness says about it.
    pub health: Option<CollectionHealth>,
}

impl Row {
    /// Vectors held, or None for a collection that has not been opened.
    pub fn vectors(&self) -> Option<usize> {
        self.metrics.as_ref().map(|m| m.vector_count)
    }

    /// Vector width, or None when the collection is not open or stores no vector yet.
    pub fn dimension(&self) -> Option<usize> {
        self.info.as_ref().and_then(|info| info.dimensions)
    }

    /// The metric the collection scores with, or None when the collection is not open.
    pub fn metric(&self) -> Option<&str> {
        self.info.as_ref().map(|info| info.metric.as_str())
    }

    /// Whether the server has this collection open.
    pub fn loaded(&self) -> bool {
        self.health.as_ref().is_some_and(|h| h.loaded) || self.metrics.is_some()
    }

    /// The problem readiness reported, if any.
    pub fn problem(&self) -> Option<&str> {
        let health = self.health.as_ref()?;
        if let Some(error) = health.error.as_deref() {
            return Some(error);
        }
        (health.integrity_ok == Some(false)).then_some("integrity check failed")
    }
}

/// An action that changes the server, held until it is confirmed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pending {
    /// Compact the collection, reclaiming space held by deleted records.
    Compact(String),
}

impl Pending {
    /// The question to put on the confirmation line.
    pub fn question(&self) -> String {
        match self {
            Self::Compact(name) => {
                format!("compact {name}? it rewrites the record store  [y/n]")
            }
        }
    }
}

/// What a key press in the collections view asks the console to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    /// Nothing beyond the change to the view state.
    Nothing,
    /// Show this line in the status bar notice.
    Notice(String),
    /// Run this confirmed action.
    Dispatch(Pending),
}

/// Collections view state.
pub struct Collections {
    /// The server this dashboard talks to.
    pub client: Client,
    /// Version string for the status bar, empty until the version endpoint answers.
    pub version: String,
    /// Rows in display order.
    pub rows: Vec<Row>,
    /// Index into rows.
    pub selected: usize,
    /// Server-wide totals from the last successful refresh.
    pub snapshot: Option<Snapshot>,
    /// Why the last refresh failed, if it did.
    pub error: Option<ClientError>,
    /// Search latency history in microseconds, by collection.
    pub history: HashMap<String, VecDeque<u64>>,
    /// An action waiting on a yes or no key.
    pub pending: Option<Pending>,
    /// When the last refresh landed, for the elapsed-time indicator.
    pub last_refresh: Option<Instant>,
    /// A refresh is in flight. No second refresh starts while one is.
    pub refreshing: bool,
    /// Time between refreshes.
    pub interval: Duration,
    /// First half of a two-key chord such as gg.
    pending_key: Option<char>,
}

impl Collections {
    /// A collections view over client, refreshing every interval.
    pub fn new(client: Client, interval: Duration) -> Self {
        Self {
            client,
            version: String::new(),
            rows: Vec::new(),
            selected: 0,
            snapshot: None,
            error: None,
            history: HashMap::new(),
            pending: None,
            last_refresh: None,
            refreshing: false,
            interval,
            pending_key: None,
        }
    }

    /// The selected collection, if there is one.
    pub fn current(&self) -> Option<&Row> {
        self.rows.get(self.selected)
    }

    /// Whether a refresh is due.
    pub fn refresh_due(&self) -> bool {
        !self.refreshing
            && self
                .last_refresh
                .is_none_or(|at| at.elapsed() >= self.interval)
    }

    /// Applies a key press, and reports what the console has to show or dispatch for it.
    pub fn key(&mut self, key: KeyEvent) -> Reply {
        self.on_key(key)
    }

    /// Applies a refresh result.
    pub fn snapshot(&mut self, result: Result<Snapshot, ClientError>) {
        self.refreshing = false;
        self.last_refresh = Some(Instant::now());
        match result {
            Ok(snapshot) => {
                self.error = None;
                self.rebuild_rows(&snapshot);
                self.snapshot = Some(snapshot);
            }
            // The last good snapshot stays on screen when a poll fails.
            Err(e) => self.error = Some(e),
        }
    }

    /// Folds metrics, WAL stats, the collection list and readiness into one row per collection.
    fn rebuild_rows(&mut self, snapshot: &Snapshot) {
        let selected = self.current().map(|r| r.name.clone());
        let mut rows: HashMap<String, Row> = HashMap::new();
        for health in &snapshot.ready.collections {
            rows.entry(health.name.clone()).or_default().health = Some(health.clone());
        }
        for metrics in &snapshot.metrics.collections {
            let row = rows.entry(metrics.name.clone()).or_default();
            if let Some(micros) = metrics.search_latency_ms.map(|ms| (ms * 1000.0) as u64) {
                let history = self.history.entry(metrics.name.clone()).or_default();
                if history.len() == HISTORY {
                    history.pop_front();
                }
                history.push_back(micros);
            }
            row.metrics = Some(metrics.clone());
        }
        // A durability stat or a summary attaches to an existing row and never creates one.
        for wal in &snapshot.metrics.wal_stats {
            if let Some(row) = rows.get_mut(&wal.collection) {
                row.wal = Some(wal.clone());
            }
        }
        for info in &snapshot.list.collections {
            if let Some(row) = rows.get_mut(&info.name) {
                row.info = Some(info.clone());
            }
        }
        let mut rows: Vec<Row> = rows
            .into_iter()
            .map(|(name, mut row)| {
                row.name = name;
                row
            })
            .collect();
        rows.sort_by(|a, b| a.name.cmp(&b.name));
        // The cursor stays on the collection it was on across a refresh.
        self.selected = selected
            .and_then(|name| rows.iter().position(|r| r.name == name))
            .unwrap_or(self.selected)
            .min(rows.len().saturating_sub(1));
        self.rows = rows;
    }

    fn on_key(&mut self, key: KeyEvent) -> Reply {
        if let Some(pending) = self.pending.take() {
            return match key.code {
                KeyCode::Char('y' | 'Y') => Reply::Dispatch(pending),
                _ => Reply::Notice("cancelled".into()),
            };
        }
        let chord = self.pending_key.take() == Some('g');
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => self.move_by(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_by(-1),
            KeyCode::Char('g') if chord => self.selected = 0,
            KeyCode::Char('g') => self.pending_key = Some('g'),
            KeyCode::Char('G') => self.selected = self.rows.len().saturating_sub(1),
            KeyCode::Char('R') => self.last_refresh = None,
            KeyCode::Char('c') => return self.ask(Pending::Compact),
            _ => {}
        }
        Reply::Nothing
    }

    fn ask(&mut self, make: fn(String) -> Pending) -> Reply {
        match self.current() {
            Some(row) => {
                self.pending = Some(make(row.name.clone()));
                Reply::Nothing
            }
            None => Reply::Notice("no collection selected".into()),
        }
    }

    fn move_by(&mut self, delta: isize) {
        if self.rows.is_empty() {
            return;
        }
        let last = self.rows.len() - 1;
        self.selected = self.selected.saturating_add_signed(delta).min(last);
    }
}

/// The line shown when the compaction of the collection name finished.
pub fn compacted_line(name: &str, compacted: &super::client::Compacted) -> String {
    format!(
        "compaction of {name}: {} documents, {} -> {}",
        compacted.documents,
        super::ui::bytes(compacted.bytes_before),
        super::ui::bytes(compacted.bytes_after),
    )
}

/// What to call an action while it runs.
pub fn verb(pending: &Pending) -> String {
    match pending {
        Pending::Compact(name) => format!("compaction of {name}"),
    }
}
