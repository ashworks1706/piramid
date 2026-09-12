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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_a_scalar_metric() {
        let mut registry = Registry::new();
        registry.metric(
            "piramid_up",
            "Whether the server is up.",
            MetricType::Gauge,
            1.0,
        );
        assert_eq!(
            registry.render(),
            "# HELP piramid_up Whether the server is up.\n# TYPE piramid_up gauge\npiramid_up 1\n"
        );
    }

    #[test]
    fn renders_a_family_with_labels() {
        let mut registry = Registry::new();
        registry.metric_family(
            "piramid_vectors",
            "Vectors per collection.",
            MetricType::Gauge,
            vec![
                (vec![("collection", "docs".to_string())], 12.0),
                (vec![("collection", "notes".to_string())], 3.0),
            ],
        );
        let out = registry.render();
        assert!(out.contains("piramid_vectors{collection=\"docs\"} 12\n"));
        assert!(out.contains("piramid_vectors{collection=\"notes\"} 3\n"));
        // The header appears once for the family, not once per sample.
        assert_eq!(out.matches("# TYPE").count(), 1);
    }

    #[test]
    fn omits_an_empty_family_entirely() {
        let mut registry = Registry::new();
        registry.metric_family(
            "piramid_vectors",
            "Vectors per collection.",
            MetricType::Gauge,
            Vec::<(Vec<(&str, String)>, f64)>::new(),
        );
        assert_eq!(registry.render(), "");
    }

    #[test]
    fn omits_an_absent_metric_entirely() {
        let mut registry = Registry::new();
        registry.optional_metric("piramid_host_cpu_percent", "CPU.", MetricType::Gauge, None);
        assert_eq!(registry.render(), "");

        let mut registry = Registry::new();
        registry.optional_metric(
            "piramid_host_cpu_percent",
            "CPU.",
            MetricType::Gauge,
            Some(0.0),
        );
        assert!(registry.render().contains("piramid_host_cpu_percent 0\n"));
    }

    #[test]
    fn escapes_label_values() {
        let mut registry = Registry::new();
        registry.metric_family(
            "piramid_vectors",
            "Vectors per collection.",
            MetricType::Gauge,
            vec![(vec![("collection", "a\"b\\c".to_string())], 1.0)],
        );
        assert!(registry.render().contains(r#"collection="a\"b\\c""#));
    }

    #[test]
    fn formats_fractional_values_with_a_decimal_point() {
        let mut registry = Registry::new();
        registry.metric("piramid_latency", "Latency.", MetricType::Gauge, 1.5);
        assert!(registry.render().contains("piramid_latency 1.5\n"));
    }
}
