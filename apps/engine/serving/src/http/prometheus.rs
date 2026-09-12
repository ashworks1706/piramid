//! Renders /api/metrics JSON into the Prometheus text exposition format.

use piramid_core::observability::prometheus::{MetricType, Registry};

use crate::services::api::{HostMetricsResponse, MetricsResponse};

/// Render a metrics snapshot in the Prometheus text format.
pub fn render(metrics: &MetricsResponse) -> String {
    let mut registry = Registry::new();

    registry.metric(
        "piramid_collections_total",
        "Number of collections currently loaded.",
        MetricType::Gauge,
        metrics.total_collections as f64,
    );
    registry.metric(
        "piramid_vectors_total",
        "Number of vectors across all loaded collections.",
        MetricType::Gauge,
        metrics.total_vectors as f64,
    );

    let by_collection = |extract: fn(&crate::services::api::CollectionMetrics) -> Option<f64>| {
        metrics
            .collections
            .iter()
            .filter_map(|c| extract(c).map(|value| (vec![("collection", c.name.clone())], value)))
            .collect::<Vec<_>>()
    };

    registry.metric_family(
        "piramid_collection_vectors",
        "Vectors in a collection.",
        MetricType::Gauge,
        by_collection(|c| Some(c.vector_count as f64)),
    );
    registry.metric_family(
        "piramid_collection_memory_bytes",
        "Approximate resident bytes for a collection.",
        MetricType::Gauge,
        by_collection(|c| Some(c.memory_usage_bytes as f64)),
    );
    registry.metric_family(
        "piramid_collection_insert_latency_ms",
        "Mean insert latency in milliseconds.",
        MetricType::Gauge,
        by_collection(|c| c.insert_latency_ms.map(f64::from)),
    );
    registry.metric_family(
        "piramid_collection_search_latency_ms",
        "Mean search latency in milliseconds.",
        MetricType::Gauge,
        by_collection(|c| c.search_latency_ms.map(f64::from)),
    );
    registry.metric_family(
        "piramid_collection_lock_read_ms",
        "Mean time waiting for a collection read lock, in milliseconds.",
        MetricType::Gauge,
        by_collection(|c| c.lock_read_ms.map(f64::from)),
    );
    registry.metric_family(
        "piramid_collection_lock_write_ms",
        "Mean time waiting for a collection write lock, in milliseconds.",
        MetricType::Gauge,
        by_collection(|c| c.lock_write_ms.map(f64::from)),
    );

    // The index type is published as a label on a constant-1 gauge.
    registry.metric_family(
        "piramid_collection_index_info",
        "Index family in use for a collection.",
        MetricType::Gauge,
        metrics
            .collections
            .iter()
            .map(|c| {
                (
                    vec![
                        ("collection", c.name.clone()),
                        ("index_type", c.index_type.clone()),
                    ],
                    1.0,
                )
            })
            .collect::<Vec<_>>(),
    );

    registry.metric_family(
        "piramid_wal_size_bytes",
        "Write-ahead log size for a collection.",
        MetricType::Gauge,
        metrics
            .wal_stats
            .iter()
            .filter_map(|w| {
                w.wal_size_bytes
                    .map(|bytes| (vec![("collection", w.collection.clone())], bytes as f64))
            })
            .collect::<Vec<_>>(),
    );
    registry.metric_family(
        "piramid_wal_checkpoint_age_seconds",
        "Seconds since a collection last checkpointed.",
        MetricType::Gauge,
        metrics
            .wal_stats
            .iter()
            .filter_map(|w| {
                w.checkpoint_age_secs
                    .map(|age| (vec![("collection", w.collection.clone())], age as f64))
            })
            .collect::<Vec<_>>(),
    );

    registry.metric(
        "piramid_embedding_requests_total",
        "Embedding requests issued to the provider.",
        MetricType::Counter,
        metrics.embedding.requests as f64,
    );
    registry.metric(
        "piramid_embedding_texts_total",
        "Texts submitted for embedding.",
        MetricType::Counter,
        metrics.embedding.texts as f64,
    );
    registry.metric(
        "piramid_embedding_tokens_total",
        "Tokens reported by the embedding provider.",
        MetricType::Counter,
        metrics.embedding.total_tokens as f64,
    );
    registry.optional_metric(
        "piramid_embedding_latency_ms",
        "Mean embedding request latency in milliseconds.",
        MetricType::Gauge,
        metrics.embedding.avg_latency_ms.map(f64::from),
    );

    render_host(&mut registry, &metrics.host);

    registry.render()
}

/// Write the host readings, leaving out each one the server could not measure.
fn render_host(registry: &mut Registry, host: &HostMetricsResponse) {
    registry.optional_metric(
        "piramid_host_cpu_percent",
        "Processor use across every logical CPU of the host, from 0 to 100.",
        MetricType::Gauge,
        host.cpu_percent.map(f64::from),
    );
    registry.optional_metric(
        "piramid_host_memory_used_bytes",
        "Physical memory in use on the host.",
        MetricType::Gauge,
        host.memory_used_bytes.map(|bytes| bytes as f64),
    );
    registry.optional_metric(
        "piramid_host_memory_total_bytes",
        "Physical memory installed on the host.",
        MetricType::Gauge,
        host.memory_total_bytes.map(|bytes| bytes as f64),
    );
    registry.optional_metric(
        "piramid_process_cpu_percent",
        "Processor use of the server process as a share of every logical CPU of the host, from 0 to 100.",
        MetricType::Gauge,
        host.process_cpu_percent.map(f64::from),
    );
    registry.optional_metric(
        "piramid_process_resident_memory_bytes",
        "Resident memory of the server process.",
        MetricType::Gauge,
        host.process_resident_bytes.map(|bytes| bytes as f64),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
