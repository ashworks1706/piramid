//! What /api/metrics reports.

use serde::Serialize;

/// Server-wide metrics snapshot.
#[derive(Serialize)]
pub struct MetricsResponse {
    /// Number of collections open in memory.
    pub total_collections: usize,
    /// Documents across all loaded collections.
    pub total_vectors: usize,
    /// One entry per loaded collection.
    pub collections: Vec<CollectionMetrics>,
    /// The full configuration currently in effect.
    pub app_config: piramid_core::config::Config,
    /// Write-ahead log state, one entry per loaded collection.
    pub wal_stats: Vec<WalStats>,
    /// Embedding provider usage since startup.
    pub embedding: EmbeddingMetricsResponse,
    /// Processor and memory use of the host and of the server process.
    pub host: HostMetricsResponse,
    /// Memory, utilisation and temperature of each GPU the server measured. Left out when it
    /// measured none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub gpus: Vec<GpuMetricsResponse>,
}

/// Metrics of one loaded collection.
#[derive(Serialize)]
pub struct CollectionMetrics {
    /// Collection name.
    pub name: String,
    /// Number of stored documents.
    pub vector_count: usize,
    /// Index family: Flat, HNSW or IVF.
    pub index_type: String,
    /// Approximate resident size of records, offsets, caches and index, in bytes.
    pub memory_usage_bytes: usize,
    /// Moving average of insert duration, in milliseconds. Null before any sample.
    pub insert_latency_ms: Option<f32>,
    /// Moving average of search duration, in milliseconds. Null before any sample.
    pub search_latency_ms: Option<f32>,
    /// Moving average of read-lock wait, in milliseconds. Null before any sample.
    pub lock_read_ms: Option<f32>,
    /// Moving average of write-lock wait, in milliseconds. Null before any sample.
    pub lock_write_ms: Option<f32>,
    /// Multiplier applied to k when a filter is present, from the collection config.
    pub filter_overfetch: Option<usize>,
    /// Configured HNSW candidate-list width. Null unless the index is configured as HNSW.
    pub hnsw_ef_search: Option<usize>,
    /// Configured IVF partitions to scan. Null unless the index is configured as IVF.
    pub ivf_nprobe: Option<usize>,
}

/// Write-ahead log state of one loaded collection.
#[derive(Serialize)]
pub struct WalStats {
    /// Collection name.
    pub collection: String,
    /// Time of the last checkpoint since the collection was opened, in seconds since the Unix
    /// epoch. Null when not yet checkpointed.
    pub last_checkpoint: Option<u64>,
    /// Seconds since the last checkpoint. Null whenever last_checkpoint is null.
    pub checkpoint_age_secs: Option<u64>,
    /// Size of the write-ahead log file, in bytes. Null when the file does not exist.
    pub wal_size_bytes: Option<u64>,
}

/// Embedding provider usage since startup.
#[derive(Serialize)]
pub struct EmbeddingMetricsResponse {
    /// Number of embed and text-search requests served.
    pub requests: u64,
    /// Number of texts embedded.
    pub texts: u64,
    /// Tokens consumed, as reported by the provider.
    pub total_tokens: u64,
    /// Mean duration per request, in milliseconds. Absent before any request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_latency_ms: Option<f32>,
}

/// Processor and memory use of the host and of the server process. A field the server could not
/// measure is left out.
#[derive(Debug, Default, Serialize)]
pub struct HostMetricsResponse {
    /// Processor use of the whole host, 0 to 100.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_percent: Option<f32>,
    /// Host memory in use, in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_used_bytes: Option<u64>,
    /// Host memory installed, in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_total_bytes: Option<u64>,
    /// Processor use of the server process as a share of every host CPU, 0 to 100.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_cpu_percent: Option<f32>,
    /// Resident memory of the server process, in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_resident_bytes: Option<u64>,
}

/// Memory, utilisation and temperature of one GPU. A field the server could not measure is left
/// out.
#[derive(Debug, Default, Serialize)]
pub struct GpuMetricsResponse {
    /// Index of the device as the driver enumerates it.
    pub index: u32,
    /// Product name the driver reports for the device.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Device memory in use, in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_used_bytes: Option<u64>,
    /// Device memory installed, in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_total_bytes: Option<u64>,
    /// Share of the last sample period during which a kernel ran on the device, 0 to 100.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utilization_percent: Option<f32>,
    /// Temperature of the device die, in degrees Celsius.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature_celsius: Option<f32>,
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
                total_tokens: 0,
                avg_latency_ms: None,
            },
            host: HostMetricsResponse::default(),
            gpus,
        };
        let json = serde_json::to_value(metrics(Vec::new())).unwrap();
        assert!(json.get("gpus").is_none(), "{json}");

        let json = serde_json::to_value(metrics(vec![GpuMetricsResponse::default()])).unwrap();
        assert_eq!(json["gpus"], serde_json::json!([{ "index": 0 }]));
    }
}
