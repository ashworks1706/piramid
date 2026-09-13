#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]
//! Prometheus text exposition rendering.

use piramid_core::observability::prometheus::{MetricType, Registry};

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
