#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]
//! The /api/metrics JSON shapes and their Prometheus rendering.

use piramid_core::observability::prometheus::Registry;
use piramid_serving::http::prometheus::{render_gpus, render_host};
use piramid_serving::services::api::{
    EmbeddingMetricsResponse, GpuMetricsResponse, HostMetricsResponse, MetricsResponse,
};

#[test]
fn an_unmeasured_host_field_is_left_out_of_the_json() {
    let json = serde_json::to_value(HostMetricsResponse {
        memory_total_bytes: Some(4096),
        process_cpu_percent: Some(0.0),
        ..HostMetricsResponse::default()
    })
    .unwrap();
    assert_eq!(
        json,
        serde_json::json!({ "memory_total_bytes": 4096, "process_cpu_percent": 0.0 })
    );
    assert_eq!(
        serde_json::to_value(HostMetricsResponse::default()).unwrap(),
        serde_json::json!({})
    );
}

#[test]
fn an_unmeasured_gpu_field_is_left_out_of_the_json() {
    let json = serde_json::to_value(GpuMetricsResponse {
        index: 1,
        memory_total_bytes: Some(6_000_000_000),
        temperature_celsius: Some(0.0),
        ..GpuMetricsResponse::default()
    })
    .unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "index": 1,
            "memory_total_bytes": 6_000_000_000_u64,
            "temperature_celsius": 0.0
        })
    );
    assert_eq!(
        serde_json::to_value(GpuMetricsResponse::default()).unwrap(),
        serde_json::json!({ "index": 0 })
    );
}

#[test]
fn a_server_that_measured_no_gpu_sends_no_gpus_key() {
    let metrics = |gpus| MetricsResponse {
        total_collections: 0,
        total_vectors: 0,
        collections: Vec::new(),
        app_config: piramid_core::config::Config::default(),
        wal_stats: Vec::new(),
        embedding: EmbeddingMetricsResponse {
            requests: 0,
            texts: 0,
            total_tokens: None,
            avg_latency_ms: None,
        },
        host: HostMetricsResponse::default(),
        gpus,
        gpu_budget: None,
        inference: None,
    };
    let json = serde_json::to_value(metrics(Vec::new())).unwrap();
    assert!(json.get("gpus").is_none(), "{json}");

    let json = serde_json::to_value(metrics(vec![GpuMetricsResponse::default()])).unwrap();
    assert_eq!(json["gpus"], serde_json::json!([{ "index": 0 }]));
}

const HOST_FAMILIES: [&str; 5] = [
    "piramid_host_cpu_percent",
    "piramid_host_memory_used_bytes",
    "piramid_host_memory_total_bytes",
    "piramid_process_cpu_percent",
    "piramid_process_resident_memory_bytes",
];

fn rendered(host: &HostMetricsResponse) -> String {
    let mut registry = Registry::new();
    render_host(&mut registry, host);
    registry.render()
}

#[test]
fn an_unmeasured_host_writes_no_family() {
    assert_eq!(rendered(&HostMetricsResponse::default()), "");
}

#[test]
fn a_measured_host_writes_every_family() {
    let out = rendered(&HostMetricsResponse {
        cpu_percent: Some(12.5),
        memory_used_bytes: Some(1024),
        memory_total_bytes: Some(4096),
        process_cpu_percent: Some(0.0),
        process_resident_bytes: Some(512),
    });
    for family in HOST_FAMILIES {
        assert!(
            out.contains(&format!("# TYPE {family} gauge\n")),
            "{family} in {out}"
        );
    }
    assert!(out.contains("piramid_host_cpu_percent 12.5\n"));
    assert!(out.contains("piramid_host_memory_total_bytes 4096\n"));
    assert!(out.contains("piramid_process_cpu_percent 0\n"));
}

#[test]
fn only_the_measured_fields_are_written() {
    let out = rendered(&HostMetricsResponse {
        memory_total_bytes: Some(4096),
        ..HostMetricsResponse::default()
    });
    assert!(out.contains("piramid_host_memory_total_bytes 4096\n"));
    assert!(!out.contains("piramid_host_cpu_percent"));
    assert!(!out.contains("piramid_host_memory_used_bytes"));
    assert!(!out.contains("piramid_process_"));
}

fn rendered_gpus(gpus: &[GpuMetricsResponse]) -> String {
    let mut registry = Registry::new();
    render_gpus(&mut registry, gpus);
    registry.render()
}

#[test]
fn no_measured_gpu_writes_no_family() {
    assert_eq!(rendered_gpus(&[]), "");
    assert_eq!(rendered_gpus(&[GpuMetricsResponse::default()]), "");
}

#[test]
fn a_measured_gpu_writes_every_family_labelled_by_index() {
    let out = rendered_gpus(&[GpuMetricsResponse {
        index: 1,
        name: Some("device".to_owned()),
        memory_used_bytes: Some(1024),
        memory_total_bytes: Some(4096),
        utilization_percent: Some(0.0),
        temperature_celsius: Some(54.0),
    }]);
    for family in [
        "piramid_gpu_memory_used_bytes",
        "piramid_gpu_memory_total_bytes",
        "piramid_gpu_utilization_percent",
        "piramid_gpu_temperature_celsius",
    ] {
        assert!(
            out.contains(&format!("# TYPE {family} gauge\n")),
            "{family} in {out}"
        );
    }
    assert!(out.contains("piramid_gpu_memory_total_bytes{gpu=\"1\"} 4096\n"));
    assert!(out.contains("piramid_gpu_utilization_percent{gpu=\"1\"} 0\n"));
    assert!(out.contains("piramid_gpu_temperature_celsius{gpu=\"1\"} 54\n"));
}

#[test]
fn only_the_measured_gpu_fields_are_written() {
    let out = rendered_gpus(&[
        GpuMetricsResponse {
            index: 0,
            temperature_celsius: Some(40.0),
            ..GpuMetricsResponse::default()
        },
        GpuMetricsResponse {
            index: 1,
            ..GpuMetricsResponse::default()
        },
    ]);
    assert!(out.contains("piramid_gpu_temperature_celsius{gpu=\"0\"} 40\n"));
    assert!(!out.contains("gpu=\"1\""));
    assert!(!out.contains("piramid_gpu_memory"));
    assert!(!out.contains("piramid_gpu_utilization_percent"));
}
