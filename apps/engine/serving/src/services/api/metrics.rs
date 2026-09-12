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
