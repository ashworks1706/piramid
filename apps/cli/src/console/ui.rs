//! Draws the console: status bar, unit list, log pane, command line, help.

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::{self, Marker};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Axis, Block, BorderType, Borders, Chart, Clear, Dataset, GraphType, List, ListItem, ListState,
    Paragraph, Sparkline,
};
use ratatui::Frame;

use crate::console::app::{App, UnitState};
use crate::console::client::{HostMetrics, InferenceMetrics};
use crate::console::collections::Row;
use crate::console::device::{DeviceView, Run};
use crate::console::types::{
    ConfigState, Focus, Group, Mode, Probe, Profile, Status, Stream, View,
};

const ACCENT: Color = Color::Cyan;
const DIM: Color = Color::DarkGray;
const SIDEBAR: u16 = 34;

/// Renders one frame.
pub fn draw(frame: &mut Frame, app: &mut App) {
    let [bar, body, bottom] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .areas(frame.area());
    status_bar(frame, app, bar);
    match app.view {
        View::Units => {
            let [side, right] =
                Layout::horizontal([Constraint::Length(SIDEBAR), Constraint::Min(20)]).areas(body);
            sidebar(frame, app, side);
            logs(frame, app, right);
        }
        View::Collections => collections(frame, app, body),
        View::Config => config(frame, app, body),
        View::Device => device(frame, app, body),
    }
    bottom_line(frame, app, bottom);
    if app.help {
        help(frame, app, frame.area());
    }
}

/// The collection list beside the detail of the selected one.
fn collections(frame: &mut Frame, app: &App, area: Rect) {
    let [left, right] =
        Layout::horizontal([Constraint::Length(SIDEBAR), Constraint::Min(24)]).areas(area);
    let view = &app.collections;

    let width = usize::from(left.width.saturating_sub(4)).max(12);
    let items: Vec<ListItem> = view
        .rows
        .iter()
        .map(|row| {
            let count = thousands(row.vectors());
            let name_width = width.saturating_sub(count.len() + 3);
            let (glyph, color) = match (row.problem().is_some(), row.loaded()) {
                (true, _) => ("!", Color::Red),
                (false, true) => ("*", Color::Green),
                (false, false) => ("o", DIM),
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!(" {glyph} "), Style::default().fg(color)),
                Span::raw(format!("{:<name_width$}", truncate(&row.name, name_width))),
                Span::styled(count, Style::default().fg(DIM)),
            ]))
        })
        .collect();
    let title = format!(" collections {} ", view.rows.len());
    if items.is_empty() {
        let note: Vec<Line> = match (&view.error, view.snapshot.is_some()) {
            (Some(_), _) => vec![
                Line::from(Span::styled(
                    "  no server at",
                    Style::default().fg(Color::Red),
                )),
                Line::from(Span::styled(
                    format!("  {}", app.base_url()),
                    Style::default().fg(Color::Red),
                )),
                Line::default(),
                Line::from(Span::styled(
                    "  Start one with piramid",
                    Style::default().fg(DIM),
                )),
                Line::from(Span::styled(
                    "  serve, or set console.",
                    Style::default().fg(DIM),
                )),
                Line::from(Span::styled(
                    "  base_url to watch another.",
                    Style::default().fg(DIM),
                )),
            ],
            (None, true) => vec![Line::from(Span::styled(
                "  no collections yet",
                Style::default().fg(DIM),
            ))],
            (None, false) => vec![Line::from(Span::styled(
                "  connecting",
                Style::default().fg(DIM),
            ))],
        };
        frame.render_widget(Paragraph::new(note).block(pane(&title, true)), left);
    } else {
        let list = List::new(items).block(pane(&title, true)).highlight_style(
            Style::default()
                .bg(Color::Rgb(40, 44, 52))
                .add_modifier(Modifier::BOLD),
        );
        let mut state = ListState::default().with_selected(Some(view.selected));
        frame.render_stateful_widget(list, left, &mut state);
    }

    let [detail, latency] =
        Layout::vertical([Constraint::Min(3), Constraint::Length(9)]).areas(right);
    match view.current() {
        Some(row) => {
            let title = match &row.metrics {
                Some(metrics) => format!(
                    " {} {} {} vectors ",
                    row.name,
                    metrics.index_type,
                    thousands(metrics.vector_count)
                ),
                None => format!(" {} not open ", row.name),
            };
            frame.render_widget(
                Paragraph::new(collection_detail(row)).block(pane(&title, true)),
                detail,
            );
        }
        None => frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                if view.error.is_some() {
                    "  nothing to show until a server answers"
                } else {
                    "  select a collection"
                },
                Style::default().fg(DIM),
            )))
            .block(pane(" collection ", true)),
            detail,
        ),
    }

    let history: Vec<u64> = view
        .current()
        .and_then(|row| view.history.get(&row.name))
        .map(|h| h.iter().copied().collect())
        .unwrap_or_default();
    if history.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "  no search has been measured yet",
                Style::default().fg(DIM),
            )))
            .block(pane(" search latency ", false)),
            latency,
        );
    } else {
        let peak = history.iter().copied().max().unwrap_or(0);
        let width = usize::from(latency.width.saturating_sub(2)).max(1);
        let visible = &history[history.len().saturating_sub(width)..];
        frame.render_widget(
            Sparkline::default()
                .block(pane(
                    &format!(" search latency peak {} ", micros(peak)),
                    false,
                ))
                .data(visible)
                .style(Style::default().fg(ACCENT)),
            latency,
        );
    }
}

fn collection_detail(row: &Row) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    if let Some(problem) = row.problem() {
        lines.push(Line::from(Span::styled(
            format!("  {problem}"),
            Style::default().fg(Color::Red).bold(),
        )));
        lines.push(Line::default());
    }
    let Some(metrics) = &row.metrics else {
        lines.push(Line::from(Span::styled(
            "  on disk, not open. The server loads a collection on first use.",
            Style::default().fg(DIM),
        )));
        return lines;
    };
    lines.push(heading("index"));
    lines.push(field("type", &metrics.index_type, 18));
    if let Some(ef) = metrics.hnsw_ef_search {
        lines.push(field("ef_search", &ef.to_string(), 18));
    }
    if let Some(nprobe) = metrics.ivf_nprobe {
        lines.push(field("nprobe", &nprobe.to_string(), 18));
    }
    lines.push(field(
        "memory",
        &bytes(metrics.memory_usage_bytes as u64),
        18,
    ));
    lines.push(Line::default());
    lines.push(heading("latency"));
    lines.push(field("search", &millis(metrics.search_latency_ms), 18));
    lines.push(field("insert", &millis(metrics.insert_latency_ms), 18));
    lines.push(field("lock read", &millis(metrics.lock_read_ms), 18));
    lines.push(field("lock write", &millis(metrics.lock_write_ms), 18));
    lines.push(Line::default());
    lines.push(heading("durability"));
    let (age, size) = row
        .wal
        .as_ref()
        .map_or((None, None), |w| (w.checkpoint_age_secs, w.wal_size_bytes));
    lines.push(field(
        "last checkpoint",
        &age.map_or_else(|| "never".to_owned(), |s| format!("{} ago", duration(s))),
        18,
    ));
    lines.push(field(
        "wal size",
        &size.map_or_else(|| "none".to_owned(), bytes),
        18,
    ));
    lines
}

/// Host processor and memory of the watched server, each of its GPUs, and generation on its
/// loaded model, graphed over the refresh history.
fn device(frame: &mut Frame, app: &App, area: Rect) {
    let view = &app.device;
    let inference = view.latest_inference();
    let [about, cpu_row, memory_row, inference_row] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(6),
        Constraint::Min(6),
        Constraint::Min(match inference {
            None => 0,
            Some(_) if area.width >= KV_BESIDE => KV_HEIGHT.max(GENERATION_HEIGHT),
            Some(_) => KV_HEIGHT + GENERATION_HEIGHT,
        }),
    ])
    .areas(area);
    let latest = view.latest();
    let gpu_indices = view.gpu_indices();
    let columns = vec![Constraint::Fill(1); gpu_indices.len() + 1];
    let cpu_cells = Layout::horizontal(columns.clone()).split(cpu_row);
    let memory_cells = Layout::horizontal(columns).split(memory_row);
    let ([cpu_area, gpu_compute_cells @ ..], [memory_area, gpu_memory_cells @ ..]) =
        (&cpu_cells[..], &memory_cells[..])
    else {
        return;
    };

    let where_line = if view.local() {
        Line::from(vec![
            Span::styled("  this machine  ", Style::default().fg(Color::Green)),
            Span::styled(
                "h hands the terminal to htop, n to nvtop, quitting either returns here",
                Style::default().fg(DIM),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled("  remote server  ", Style::default().fg(Color::Yellow)),
            Span::styled(
                "htop and nvtop would show this machine, not the server, so h and n are off",
                Style::default().fg(DIM),
            ),
        ])
    };
    frame.render_widget(
        Paragraph::new(where_line).block(pane(&format!(" device {} ", app.base_url()), true)),
        about,
    );

    let now = std::time::Instant::now();
    let window = view
        .samples
        .front()
        .map_or(0.0, |first| {
            now.saturating_duration_since(first.at).as_secs_f64()
        })
        .max(60.0);

    let host_cpu = view.series(now, |h| h.cpu_percent.map(f64::from));
    let process_cpu = view.series(now, |h| h.process_cpu_percent.map(f64::from));
    let cpu_title = format!(
        " cpu  host {}  piramid {} ",
        percent(latest.and_then(|h| h.cpu_percent)),
        percent(latest.and_then(|h| h.process_cpu_percent))
    );
    frame.render_widget(
        chart(
            &cpu_title,
            [
                Series {
                    runs: &host_cpu,
                    color: ACCENT,
                    name: "host",
                },
                Series {
                    runs: &process_cpu,
                    color: Color::Magenta,
                    name: "piramid",
                },
            ],
            window,
            100.0,
            ["0%".to_owned(), "50%".to_owned(), "100%".to_owned()],
        ),
        *cpu_area,
    );

    let used = view.series(now, |h| h.memory_used_bytes.map(|b| b as f64));
    let resident = view.series(now, |h| h.process_resident_bytes.map(|b| b as f64));
    let ceiling = memory_ceiling(view.samples.iter().filter_map(|s| s.host.as_ref()));
    let memory_title = format!(
        " memory  host {} of {}  piramid {} ",
        latest
            .and_then(|h| h.memory_used_bytes)
            .map_or_else(unmeasured, bytes),
        latest
            .and_then(|h| h.memory_total_bytes)
            .map_or_else(unmeasured, bytes),
        latest
            .and_then(|h| h.process_resident_bytes)
            .map_or_else(unmeasured, bytes)
    );
    frame.render_widget(
        chart(
            &memory_title,
            [
                Series {
                    runs: &used,
                    color: ACCENT,
                    name: "host",
                },
                Series {
                    runs: &resident,
                    color: Color::Magenta,
                    name: "piramid",
                },
            ],
            window,
            ceiling,
            [
                "0".to_owned(),
                bytes((ceiling / 2.0) as u64),
                bytes(ceiling as u64),
            ],
        ),
        *memory_area,
    );

    for ((index, compute_area), memory_area) in gpu_indices
        .iter()
        .zip(gpu_compute_cells)
        .zip(gpu_memory_cells)
    {
        gpu(
            frame,
            view,
            *index,
            now,
            window,
            *compute_area,
            *memory_area,
        );
    }

    if let Some(latest) = inference {
        generation(frame, view, latest, now, window, inference_row);
    }
}

/// Width of the key/value cache panel beside the generation chart.
const KV_PANEL: u16 = 52;

/// Rows of the key/value cache panel, borders included.
const KV_HEIGHT: u16 = 6;

/// Fewest rows of the generation chart, borders included.
const GENERATION_HEIGHT: u16 = 7;

/// Terminal widths below this stack the generation chart above the key/value cache panel.
const KV_BESIDE: u16 = 100;

/// Decode rate and time to first token graphed over the refresh history, beside the key/value
/// cache and scheduler state of the newest refresh.
fn generation(
    frame: &mut Frame,
    view: &DeviceView,
    latest: &InferenceMetrics,
    now: std::time::Instant,
    window: f64,
    area: Rect,
) {
    let [chart_area, kv_area] = if area.width >= KV_BESIDE {
        Layout::horizontal([Constraint::Fill(1), Constraint::Length(KV_PANEL)]).areas(area)
    } else {
        Layout::vertical([
            Constraint::Min(GENERATION_HEIGHT),
            Constraint::Length(KV_HEIGHT),
        ])
        .areas(area)
    };

    let decode = view.inference_series(now, |i| i.decode_tokens_per_second.map(f64::from));
    let first_token = view.inference_series(now, |i| i.avg_time_to_first_token_ms.map(f64::from));
    let top = decode
        .iter()
        .chain(first_token.iter())
        .flatten()
        .map(|(_, value)| *value)
        .fold(1.0, f64::max);
    let title = format!(
        " generation  decode {}  first token {} ",
        latest
            .decode_tokens_per_second
            .map_or_else(unmeasured, |v| format!("{v:.1} tok/s")),
        latest
            .avg_time_to_first_token_ms
            .map_or_else(unmeasured, |v| format!("{v:.0} ms"))
    );
    frame.render_widget(
        chart(
            &title,
            [
                Series {
                    runs: &decode,
                    color: ACCENT,
                    name: "decode tok/s",
                },
                Series {
                    runs: &first_token,
                    color: Color::Yellow,
                    name: "first token ms",
                },
            ],
            window,
            top,
            [
                "0".to_owned(),
                format!("{:.0}", top / 2.0),
                format!("{top:.0}"),
            ],
        ),
        chart_area,
    );

    let block = pane(&format!(" {} on {} ", latest.model, latest.device), false);
    let width = usize::from(block.inner(kv_area).width.saturating_sub(4));
    let free = latest
        .kv_blocks_total
        .saturating_sub(latest.kv_blocks_used)
        .saturating_sub(latest.kv_blocks_cached);
    let [used_cells, cached_cells, free_cells] = kv_cells(
        latest.kv_blocks_used,
        latest.kv_blocks_cached,
        latest.kv_blocks_total,
        width,
    );
    let lines = vec![
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                symbols::block::FULL.repeat(used_cells),
                Style::default().fg(ACCENT),
            ),
            Span::styled(
                symbols::block::FULL.repeat(cached_cells),
                Style::default().fg(Color::Magenta),
            ),
            Span::styled(
                symbols::shade::LIGHT.repeat(free_cells),
                Style::default().fg(DIM),
            ),
        ]),
        Line::from(vec![
            Span::styled("  used ", Style::default().fg(ACCENT)),
            Span::raw(thousands_u64(latest.kv_blocks_used)),
            Span::styled("  cached ", Style::default().fg(Color::Magenta)),
            Span::raw(thousands_u64(latest.kv_blocks_cached)),
            Span::styled("  free ", Style::default().fg(DIM)),
            Span::raw(thousands_u64(free)),
            Span::styled("  of ", Style::default().fg(DIM)),
            Span::raw(thousands_u64(latest.kv_blocks_total)),
        ]),
        Line::from(vec![
            Span::styled("  prefix hits ", Style::default().fg(DIM)),
            Span::raw(percent(latest.prefix_hit_rate.map(|rate| rate * 100.0))),
            Span::styled("  evictions ", Style::default().fg(DIM)),
            Span::raw(thousands_u64(latest.kv_evictions)),
        ]),
        Line::from(vec![
            Span::styled("  queue ", Style::default().fg(DIM)),
            Span::raw(thousands_u64(latest.queue_depth)),
            Span::styled("  running ", Style::default().fg(DIM)),
            Span::raw(thousands_u64(latest.running)),
            Span::styled("  batch ", Style::default().fg(DIM)),
            Span::raw(thousands_u64(latest.last_batch_size)),
            Span::styled("  preempted ", Style::default().fg(DIM)),
            Span::raw(thousands_u64(latest.preemptions)),
        ]),
    ];
    frame.render_widget(Paragraph::new(lines).block(block), kv_area);
}

/// Cells of a bar width wide given to used, cached and free key/value blocks out of total.
pub fn kv_cells(used: u64, cached: u64, total: u64, width: usize) -> [usize; 3] {
    if total == 0 {
        return [0, 0, width];
    }
    let cells = |blocks: u64| -> usize {
        let share = u128::from(blocks.min(total)) * width as u128;
        let rounded = (share + u128::from(total) / 2) / u128::from(total);
        usize::try_from(rounded).unwrap_or(width).min(width)
    };
    let used_cells = cells(used);
    let held_cells = cells(used.saturating_add(cached)).max(used_cells);
    [used_cells, held_cells - used_cells, width - held_cells]
}

/// Utilisation, temperature and memory of the GPU at index, graphed over the refresh history.
/// Utilisation and temperature are drawn in compute_area and device memory in memory_area.
fn gpu(
    frame: &mut Frame,
    view: &DeviceView,
    index: u32,
    now: std::time::Instant,
    window: f64,
    compute_area: Rect,
    memory_area: Rect,
) {
    let latest = view.latest_gpu(index);
    let label = latest.and_then(|g| g.name.as_deref()).map_or_else(
        || format!("gpu {index}"),
        |name| format!("gpu {index} {name}"),
    );

    let busy = view.gpu_series(now, index, |g| g.utilization_percent.map(f64::from));
    let temperature = view.gpu_series(now, index, |g| g.temperature_celsius.map(f64::from));
    let top = temperature
        .iter()
        .flatten()
        .map(|(_, celsius)| *celsius)
        .fold(100.0, f64::max);
    let compute_title = format!(
        " {label}  busy {}  temperature {} ",
        percent(latest.and_then(|g| g.utilization_percent)),
        celsius(latest.and_then(|g| g.temperature_celsius))
    );
    frame.render_widget(
        chart(
            &compute_title,
            [
                Series {
                    runs: &busy,
                    color: ACCENT,
                    name: "busy %",
                },
                Series {
                    runs: &temperature,
                    color: Color::Red,
                    name: "temperature C",
                },
            ],
            window,
            top,
            [
                "0".to_owned(),
                format!("{:.0}", top / 2.0),
                format!("{top:.0}"),
            ],
        ),
        compute_area,
    );

    let used = view.gpu_series(now, index, |g| g.memory_used_bytes.map(|b| b as f64));
    let ceiling = view
        .samples
        .iter()
        .flat_map(|sample| sample.gpus.iter())
        .filter(|g| g.index == index)
        .filter_map(|g| g.memory_total_bytes.or(g.memory_used_bytes))
        .max()
        .map_or(1.0, |top| (top as f64).max(1.0));
    let memory_title = format!(
        " {label} memory {} of {} ",
        latest
            .and_then(|g| g.memory_used_bytes)
            .map_or_else(unmeasured, bytes),
        latest
            .and_then(|g| g.memory_total_bytes)
            .map_or_else(unmeasured, bytes)
    );
    frame.render_widget(
        chart(
            &memory_title,
            [Series {
                runs: &used,
                color: ACCENT,
                name: "used",
            }],
            window,
            ceiling,
            [
                "0".to_owned(),
                bytes((ceiling / 2.0) as u64),
                bytes(ceiling as u64),
            ],
        ),
        memory_area,
    );
}

/// One named reading drawn as a set of runs.
struct Series<'a> {
    runs: &'a [Run],
    color: Color,
    name: &'static str,
}

/// A line chart of the series over the last window seconds, from zero to ceiling.
fn chart<'a, const N: usize>(
    title: &str,
    series: [Series<'a>; N],
    window: f64,
    ceiling: f64,
    y_labels: [String; 3],
) -> Chart<'a> {
    let datasets: Vec<Dataset<'a>> = series
        .into_iter()
        .flat_map(|Series { runs, color, name }| {
            runs.iter().enumerate().map(move |(index, run)| {
                let dataset = Dataset::default()
                    .marker(Marker::Braille)
                    .graph_type(GraphType::Line)
                    .style(Style::default().fg(color))
                    .data(run);
                if index == 0 {
                    dataset.name(name)
                } else {
                    dataset
                }
            })
        })
        .collect();
    Chart::new(datasets)
        .block(pane(title, false))
        .x_axis(
            Axis::default()
                .style(Style::default().fg(DIM))
                .bounds([-window, 0.0])
                .labels([format!("-{}", duration(window as u64)), "now".to_owned()]),
        )
        .y_axis(
            Axis::default()
                .style(Style::default().fg(DIM))
                .bounds([0.0, ceiling])
                .labels(y_labels),
        )
}

/// The top of the memory axis: the largest total the server reported, else the largest reading.
fn memory_ceiling<'a>(readings: impl Iterator<Item = &'a HostMetrics>) -> f64 {
    let top = readings
        .filter_map(|host| {
            host.memory_total_bytes
                .or(host.memory_used_bytes)
                .or(host.process_resident_bytes)
        })
        .max()
        .unwrap_or(0);
    (top as f64).max(1.0)
}

/// A percentage reading, or a phrase saying it was not measured.
fn percent(value: Option<f32>) -> String {
    value.map_or_else(unmeasured, |v| format!("{v:.1}%"))
}

/// A temperature reading in degrees Celsius, or a phrase saying it was not measured.
fn celsius(value: Option<f32>) -> String {
    value.map_or_else(unmeasured, |v| format!("{v:.0} C"))
}

/// The phrase shown in place of a reading the server did not report.
fn unmeasured() -> String {
    "not reported".to_owned()
}

/// The configuration as the server resolved it.
fn config(frame: &mut Frame, app: &App, area: Rect) {
    let body = match &app.config {
        Some(ConfigState::Loading) => Paragraph::new(Line::from(Span::styled(
            "  reading the configuration from the server",
            Style::default().fg(DIM),
        ))),
        Some(ConfigState::Loaded(text)) => {
            let rows = usize::from(area.height.saturating_sub(2)).max(1);
            let lines: Vec<Line> = text
                .lines()
                .skip(app.config_scroll)
                .take(rows)
                .map(|line| {
                    let indent = line.len() - line.trim_start().len();
                    let style = if line.trim_end().ends_with(':') {
                        Style::default().fg(ACCENT)
                    } else {
                        Style::default()
                    };
                    Line::from(vec![
                        Span::raw(" ".repeat(2 + indent)),
                        Span::styled(line.trim_start().to_owned(), style),
                    ])
                })
                .collect();
            Paragraph::new(lines)
        }
        Some(ConfigState::Failed(why)) => Paragraph::new(Line::from(Span::styled(
            format!("  {why}"),
            Style::default().fg(Color::Red),
        ))),
        None => Paragraph::new(Line::from(Span::styled(
            "  not loaded",
            Style::default().fg(DIM),
        ))),
    };
    frame.render_widget(body.block(pane(" config ", true)), area);
}

fn status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let mode = match app.mode {
        Mode::Normal => " NORMAL ",
        Mode::Command => " COMMAND ",
        Mode::Search => " SEARCH ",
    };
    let mut spans = vec![
        Span::styled(
            " piramid ",
            Style::default().fg(Color::Black).bg(ACCENT).bold(),
        ),
        Span::styled(mode, Style::default().fg(Color::Black).bg(Color::White)),
    ];
    // One digit per view, so the tabs are also their own key hints.
    for (index, view) in app.profile.views().iter().enumerate() {
        let selected = *view == app.view;
        let style = if selected {
            Style::default().fg(Color::Black).bg(Color::White).bold()
        } else {
            Style::default().fg(DIM)
        };
        spans.push(Span::styled(
            format!(" {} {} ", index + 1, view.title()),
            style,
        ));
    }
    if !app.collections.version.is_empty() {
        spans.push(Span::styled(
            format!(" {} ", app.collections.version),
            Style::default().fg(DIM),
        ));
    }
    spans.push(probe_span("server", &app.health.live));
    spans.push(probe_span("ready", &app.health.ready));
    if app.profile == Profile::Developer {
        spans.push(probe_span("web", &app.health.web));
    }
    // The notice comes first so a long line of probe reasons cannot push it off the bar.
    if let Some(notice) = &app.notice {
        spans.push(Span::styled(
            format!("  {notice}"),
            Style::default().fg(Color::Magenta),
        ));
    }
    for (name, probe) in probe_problems(app) {
        let (why, color) = match probe {
            Probe::Degraded(why) => (why, Color::Yellow),
            Probe::Down(why) => (why, Color::Red),
            Probe::Unknown | Probe::Up => continue,
        };
        spans.push(Span::styled(
            format!("  {name}: {}", truncate(why, 80)),
            Style::default().fg(color),
        ));
    }
    if let Some(why) = &app.probes_stopped {
        spans.push(Span::styled(
            format!("  {why}"),
            Style::default().fg(Color::Red),
        ));
    }
    // A failed refresh is the whole story on a console that only watches a server, so it goes in
    // the bar rather than staying inside the view that collected it.
    if let Some(error) = &app.collections.error {
        spans.push(Span::styled(
            format!("  {error}"),
            Style::default().fg(Color::Red),
        ));
    }
    frame.render_widget(Line::from(spans), area);
}

/// The probes whose reason goes on the status bar, by name.
///
/// Readiness is left out while liveness is down or degraded.
fn probe_problems(app: &App) -> Vec<(&'static str, &Probe)> {
    let mut problems = Vec::new();
    match &app.health.live {
        Probe::Down(_) | Probe::Degraded(_) => problems.push(("server", &app.health.live)),
        Probe::Unknown | Probe::Up => problems.push(("ready", &app.health.ready)),
    }
    if app.profile == Profile::Developer {
        problems.push(("web", &app.health.web));
    }
    problems
}

fn probe_span(name: &str, probe: &Probe) -> Span<'static> {
    let (glyph, color) = match probe {
        Probe::Unknown => ("·", DIM),
        Probe::Up => ("●", Color::Green),
        Probe::Degraded(_) => ("◐", Color::Yellow),
        Probe::Down(_) => ("○", Color::Red),
    };
    Span::styled(format!(" {glyph} {name}"), Style::default().fg(color))
}

fn status_style(status: &Status) -> Style {
    match status {
        Status::Stopped => Style::default().fg(DIM),
        Status::Starting => Style::default().fg(Color::Yellow),
        Status::Running => Style::default().fg(Color::Green),
        Status::Exited(0) => Style::default().fg(Color::Blue),
        Status::Exited(_) | Status::Failed(_) => Style::default().fg(Color::Red),
    }
}

fn sidebar(frame: &mut Frame, app: &App, area: Rect) {
    let mut items: Vec<ListItem> = Vec::new();
    let mut selected_row = 0;
    for group in Group::ALL {
        let members: Vec<(usize, &UnitState)> = app
            .units
            .iter()
            .enumerate()
            .filter(|(_, state)| state.unit.group == group)
            .collect();
        if members.is_empty() {
            continue;
        }
        items.push(ListItem::new(Line::from(Span::styled(
            format!(" {}", group.title()),
            Style::default().fg(DIM).add_modifier(Modifier::BOLD),
        ))));
        for (index, state) in members {
            if index == app.selected {
                selected_row = items.len();
            }
            let port = state
                .unit
                .url
                .as_deref()
                .and_then(|url| url.rsplit(':').next())
                .filter(|port| port.chars().all(|c| c.is_ascii_digit()))
                .map(|port| format!(":{port}"))
                .unwrap_or_default();
            let width = usize::from(SIDEBAR).saturating_sub(6 + port.len());
            items.push(ListItem::new(Line::from(vec![
                Span::styled(
                    format!("  {} ", state.status.glyph()),
                    status_style(&state.status),
                ),
                Span::raw(format!("{:<width$}", truncate(&state.unit.id, width))),
                Span::styled(port, Style::default().fg(DIM)),
            ])));
        }
    }
    let list = List::new(items)
        .block(pane(" units ", app.focus == Focus::Units))
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(40, 44, 52))
                .add_modifier(Modifier::BOLD),
        );
    let mut state = ListState::default().with_selected(Some(selected_row));
    frame.render_stateful_widget(list, area, &mut state);
}

fn logs(frame: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focus == Focus::Logs;
    let rows = usize::from(area.height.saturating_sub(2)).max(1);
    app.log_rows = rows;
    let search = app.search.to_lowercase();
    let hit = app.search_hit;
    let state = app.current();
    let elapsed = state
        .started_at
        .filter(|_| state.status.is_active())
        .map(|at| format!(" · {}s", at.elapsed().as_secs()))
        .unwrap_or_default();
    let title = format!(
        " {} · {}{} · {} lines{} ",
        state.unit.id,
        state.status.label(),
        elapsed,
        state.logs.len(),
        if state.follow { " · follow" } else { "" }
    );
    let top = if state.follow {
        state.logs.len().saturating_sub(rows)
    } else {
        state.scroll.min(state.logs.len().saturating_sub(rows))
    };
    let lines: Vec<Line> = state
        .logs
        .lines()
        .enumerate()
        .skip(top)
        .take(rows)
        .map(|(index, line)| {
            let mut style = match line.stream {
                Stream::Out => Style::default(),
                Stream::Err => Style::default().fg(Color::Gray),
                Stream::Meta => Style::default().fg(ACCENT).italic(),
            };
            if !search.is_empty() && line.text.to_lowercase().contains(&search) {
                style = style.fg(Color::Yellow);
                if hit == Some(index) {
                    style = style.add_modifier(Modifier::BOLD | Modifier::REVERSED);
                }
            }
            Line::from(vec![
                Span::styled(
                    line.at.format("%H:%M:%S ").to_string(),
                    Style::default().fg(DIM),
                ),
                Span::styled(line.text.clone(), style),
            ])
        })
        .collect();
    let block = pane(&title, focused).title_bottom(
        Line::from(Span::styled(
            format!(" {} ", state.unit.hint),
            Style::default().fg(DIM),
        ))
        .right_aligned(),
    );
    let body = if state.logs.is_empty() {
        Paragraph::new(Line::from(Span::styled(
            "  nothing captured yet — press enter to start",
            Style::default().fg(DIM),
        )))
    } else {
        Paragraph::new(lines)
    };
    frame.render_widget(body.block(block), area);
}

fn bottom_line(frame: &mut Frame, app: &App, area: Rect) {
    // A confirmation takes the line over, whichever view raised it.
    if let Some(pending) = &app.collections.pending {
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    " confirm ",
                    Style::default().fg(Color::Black).bg(Color::Yellow),
                ),
                Span::styled(
                    format!(" {}", pending.question()),
                    Style::default().fg(Color::Yellow),
                ),
            ])),
            area,
        );
        return;
    }
    let line = match app.mode {
        Mode::Command => Line::from(vec![
            Span::styled(":", Style::default().fg(ACCENT)),
            Span::raw(app.input.clone()),
            Span::styled("█", Style::default().fg(ACCENT)),
        ]),
        Mode::Search => Line::from(vec![
            Span::styled("/", Style::default().fg(Color::Yellow)),
            Span::raw(app.input.clone()),
            Span::styled("█", Style::default().fg(Color::Yellow)),
        ]),
        Mode::Normal => {
            let mut spans: Vec<Span> = Vec::new();
            let hints: &[(&str, &str)] = match app.view {
                View::Units => &[
                    ("j/k", "move"),
                    ("⏎", "start/stop"),
                    ("r", "restart"),
                    ("l/h", "logs/units"),
                    ("/", "search"),
                    (":", "command"),
                    ("o", "open url"),
                ],
                View::Collections => &[
                    ("j/k", "move"),
                    ("r", "rebuild index"),
                    ("c", "compact"),
                    ("R", "refresh"),
                ],
                View::Config => &[("j/k", "scroll"), ("g", "top"), ("R", "reload")],
                View::Device => &[("h", "htop"), ("n", "nvtop"), ("R", "refresh")],
            };
            for (key, what) in hints.iter().copied().chain([("?", "help"), ("q", "quit")]) {
                spans.push(Span::styled(
                    format!(" {key}"),
                    Style::default().fg(ACCENT).bold(),
                ));
                spans.push(Span::styled(format!(" {what}"), Style::default().fg(DIM)));
            }
            spans.push(Span::styled(
                match app.profile {
                    Profile::Developer => {
                        format!("   {} · {}", app.base_url(), app.web_url())
                    }
                    Profile::Production => format!("   {}", app.base_url()),
                },
                Style::default().fg(DIM),
            ));
            Line::from(spans)
        }
    };
    frame.render_widget(Paragraph::new(line), area);
}

fn help(frame: &mut Frame, app: &App, area: Rect) {
    let shared: &[(&str, &str)] = &[
        ("1 to 9", "switch view, numbered as in the bar above"),
        ("?", "this help"),
        ("q, ctrl-c", "quit"),
    ];
    let per_view: &[(&str, &str)] = match app.view {
        View::Units => &[
            ("j / k", "move selection, or scroll the log pane"),
            ("gg / G", "top, bottom. G on logs resumes following"),
            ("ctrl-d / ctrl-u", "half page in logs"),
            ("enter / s", "start or stop the selected unit"),
            ("x / r", "stop, restart the selected unit"),
            ("h / l, tab", "focus units, logs"),
            ("/ then n / N", "search the logs of the selected unit"),
            ("C", "clear the logs of the selected unit"),
            ("o", "open the URL of the unit in a browser"),
            (":start x  :stop x", "act on a unit by name"),
            (":<recipe> [args]", "run any just recipe"),
        ],
        View::Collections => &[
            ("j / k", "move between collections"),
            ("gg / G", "first, last collection"),
            (
                "r",
                "rebuild the index of the selected collection, after y or n",
            ),
            ("c", "compact the selected collection, after y or n"),
            ("R", "refresh now instead of waiting for the interval"),
        ],
        View::Config => &[
            ("j / k", "scroll"),
            ("g", "top"),
            ("R", "read the configuration again"),
        ],
        View::Device => &[
            ("h", "hand the terminal to htop on this machine"),
            ("n", "hand the terminal to nvtop on this machine"),
            ("R", "refresh now instead of waiting for the interval"),
        ],
    };

    let mut lines = vec![
        Line::from(Span::styled(
            format!("{} view", app.view.title()),
            Style::default().fg(ACCENT).bold(),
        )),
        Line::default(),
    ];
    for (key, what) in per_view.iter().chain(shared) {
        lines.push(Line::from(vec![
            Span::styled(format!("  {key:<24}"), Style::default().fg(Color::Yellow)),
            Span::raw((*what).to_owned()),
        ]));
    }
    lines.push(Line::default());
    lines.push(Line::from(Span::styled(
        match app.profile {
            Profile::Developer => {
                "  Running inside a checkout, so the units view can drive the repo."
            }
            Profile::Production => {
                "  Running outside a checkout. The units view needs a justfile and is hidden."
            }
        },
        Style::default().fg(DIM),
    )));
    lines.push(Line::default());
    lines.push(Line::from(Span::styled(
        "any key closes this",
        Style::default().fg(DIM),
    )));

    let width = 88.min(area.width.saturating_sub(4));
    let height = u16::try_from(lines.len() + 2)
        .unwrap_or(u16::MAX)
        .min(area.height.saturating_sub(2));
    let popup = Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    };
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(lines)
            .block(pane(" help ", true))
            .alignment(Alignment::Left),
        popup,
    );
}

fn pane(title: &str, focused: bool) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if focused { ACCENT } else { DIM }))
        .title(Span::styled(
            title.to_owned(),
            Style::default().fg(Color::White).bold(),
        ))
}

fn truncate(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_owned();
    }
    let mut out: String = text.chars().take(width.saturating_sub(1)).collect();
    out.push('…');
    out
}

fn heading(text: &str) -> Line<'static> {
    Line::from(Span::styled(
        format!("  {text}"),
        Style::default().fg(DIM).add_modifier(Modifier::BOLD),
    ))
}

fn field(key: &str, value: &str, width: usize) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {key:<width$}"), Style::default().fg(DIM)),
        Span::raw(value.to_owned()),
    ])
}

/// A count with thousands separators.
pub fn thousands(n: usize) -> String {
    thousands_u64(n as u64)
}

/// A u64 count with thousands separators.
pub fn thousands_u64(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// A byte count at the largest unit that keeps it under four digits.
pub fn bytes(n: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = n as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{n} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// A millisecond measurement, or a placeholder when nothing has been measured.
pub fn millis(ms: Option<f32>) -> String {
    ms.map_or_else(|| "none".to_owned(), |v| format!("{v:.2} ms"))
}

/// A microsecond measurement rendered in milliseconds.
pub fn micros(us: u64) -> String {
    format!("{:.2} ms", us as f64 / 1000.0)
}

/// A span of seconds at the coarsest unit that still says something.
pub fn duration(secs: u64) -> String {
    match secs {
        0..=59 => format!("{secs}s"),
        60..=3599 => format!("{}m", secs / 60),
        3600..=86_399 => format!("{}h", secs / 3600),
        _ => format!("{}d", secs / 86_400),
    }
}
