//! What /api/metrics reports.

use serde::Serialize;

#[derive(Serialize)]
pub struct MetricsResponse {
    pub total_collections: usize,
    pub total_vectors: usize,
    pub collections: Vec<CollectionMetrics>,
    pub app_config: piramid_core::config::Config,
    pub wal_stats: Vec<WalStats>,
    pub embedding: EmbeddingMetricsResponse,
    pub host: HostMetricsResponse,
}

#[derive(Serialize)]
pub struct CollectionMetrics {
    pub name: String,
    pub vector_count: usize,
    pub index_type: String,
    pub memory_usage_bytes: usize,
    pub insert_latency_ms: Option<f32>,
    pub search_latency_ms: Option<f32>,
    pub lock_read_ms: Option<f32>,
    pub lock_write_ms: Option<f32>,
    pub filter_overfetch: Option<usize>,
    pub hnsw_ef_search: Option<usize>,
    pub ivf_nprobe: Option<usize>,
}

#[derive(Serialize)]
pub struct WalStats {
    pub collection: String,
    pub last_checkpoint: Option<u64>,
    pub checkpoint_age_secs: Option<u64>,
    pub wal_size_bytes: Option<u64>,
}

#[derive(Serialize)]
pub struct EmbeddingMetricsResponse {
    pub requests: u64,
    pub texts: u64,
    pub total_tokens: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_latency_ms: Option<f32>,
}

/// Processor and memory use of the host and of the server process. A field the server could not
/// measure is left out.
#[derive(Debug, Default, Serialize)]
pub struct HostMetricsResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_percent: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_used_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_total_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_cpu_percent: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_resident_bytes: Option<u64>,
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "a failed assertion is the point of a test"
)]
mod tests {
    use super::*;

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
}
