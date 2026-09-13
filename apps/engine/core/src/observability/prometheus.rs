//! Prometheus text exposition format: <https://prometheus.io/docs/instrumenting/exposition_formats/>

use std::fmt::Write;

/// Metric type, as declared in a TYPE line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricType {
    /// Monotonically increasing total.
    Counter,
    /// Value that can go up or down.
    Gauge,
}

impl MetricType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Counter => "counter",
            Self::Gauge => "gauge",
        }
    }
}

/// Accumulates metrics and renders them in the Prometheus text format.
#[derive(Debug, Default)]
pub struct Registry {
    out: String,
}

impl Registry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Write a metric with no labels.
    pub fn metric(&mut self, name: &str, help: &str, kind: MetricType, value: f64) {
        self.header(name, help, kind);
        let _ = writeln!(self.out, "{name} {}", format_value(value));
    }

    /// Write a metric with no labels when it has a value, and nothing when it has none.
    pub fn optional_metric(
        &mut self,
        name: &str,
        help: &str,
        kind: MetricType,
        value: Option<f64>,
    ) {
        if let Some(value) = value {
            self.metric(name, help, kind, value);
        }
    }

    /// Write a metric family, one line per label set.
    pub fn metric_family<'a>(
        &mut self,
        name: &str,
        help: &str,
        kind: MetricType,
        samples: impl IntoIterator<Item = (Vec<(&'a str, String)>, f64)>,
    ) {
        let mut wrote_header = false;
        for (labels, value) in samples {
            if !wrote_header {
                self.header(name, help, kind);
                wrote_header = true;
            }
            let rendered: Vec<String> = labels
                .iter()
                .map(|(key, value)| format!("{key}=\"{}\"", escape(value)))
                .collect();
            let _ = writeln!(
                self.out,
                "{name}{{{}}} {}",
                rendered.join(","),
                format_value(value)
            );
        }
    }

    /// Finish and return the exposition body.
    pub fn render(self) -> String {
        self.out
    }

    fn header(&mut self, name: &str, help: &str, kind: MetricType) {
        let _ = writeln!(self.out, "# HELP {name} {help}");
        let _ = writeln!(self.out, "# TYPE {name} {}", kind.as_str());
    }
}

/// The Content-Type a Prometheus scrape expects.
pub const CONTENT_TYPE: &str = "text/plain; version=0.0.4; charset=utf-8";

/// Render a float the way Prometheus expects: integers without a decimal point.
fn format_value(value: f64) -> String {
    if value.is_finite() && value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

/// Escape a label value: backslash, double quote, and newline.
fn escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}
