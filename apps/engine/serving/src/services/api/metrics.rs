//! What /api/metrics reports.

use serde::Serialize;

/// Server-wide metrics snapshot.
#[derive(Debug, Serialize)]
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
    /// How the device memory budget is divided and used. Left out when no GPU is open.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_budget: Option<GpuBudgetResponse>,
    /// Generation counters and the state of the scheduler and key/value cache. Left out when no
    /// model is loaded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inference: Option<InferenceMetricsResponse>,
}

/// The device memory budget.
#[derive(Debug, Default, Serialize)]
pub struct GpuBudgetResponse {
    /// Bytes the budget covers after the reserve.
    pub usable_bytes: u64,
    /// Whether every pool draws from one shared budget.
    pub shared: bool,
    /// Capacity and use of each pool: weights, kv_cache and index.
    pub pools: Vec<GpuPoolResponse>,
}

/// One pool of the device memory budget.
#[derive(Debug, Default, Serialize)]
pub struct GpuPoolResponse {
    /// weights, kv_cache or index.
    pub pool: &'static str,
    /// Bytes the pool may hold; under a shared budget, the whole budget.
    pub capacity_bytes: u64,
    /// Bytes reserved in the pool.
    pub used_bytes: u64,
}

/// Generation counters of the loaded model. An average not yet measured is left out.
#[derive(Debug, Default, Serialize)]
pub struct InferenceMetricsResponse {
    /// Checkpoint name of the loaded model.
    pub model: String,
    /// Device the model runs on.
    pub device: String,
    /// Requests accepted into the queue.
    pub requests_admitted: u64,
    /// Requests that ended with a finish reason.
    pub requests_finished: u64,
    /// Requests that ended with an error.
    pub requests_failed: u64,
    /// Prompt tokens across admitted requests.
    pub prompt_tokens: u64,
    /// Prompt tokens served from shared cache pages.
    pub cached_prompt_tokens: u64,
    /// Tokens generated.
    pub generated_tokens: u64,
    /// Mean time from admission to the first token, in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_time_to_first_token_ms: Option<f32>,
    /// Decode tokens per second.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decode_tokens_per_second: Option<f32>,
    /// Mean decode step duration, in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_decode_step_ms: Option<f32>,
    /// Prefill tokens per second.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefill_tokens_per_second: Option<f32>,
    /// Sequences preempted for recompute.
    pub preemptions: u64,
    /// Requests waiting for admission.
    pub queue_depth: u64,
    /// Sequences being generated.
    pub running: u64,
    /// Sequences in the most recent step.
    pub last_batch_size: u64,
    /// Pages in the key/value pool.
    pub kv_blocks_total: u64,
    /// Pages held by a sequence.
    pub kv_blocks_used: u64,
    /// Free pages still carrying a reusable prefix.
    pub kv_blocks_cached: u64,
    /// Prefix pages evicted to make room.
    pub kv_evictions: u64,
    /// Share of looked-up prompt tokens served from shared pages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix_hit_rate: Option<f32>,
}

/// Metrics of one loaded collection.
#[derive(Debug, Serialize)]
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
#[derive(Debug, Serialize)]
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
#[derive(Debug, Serialize)]
pub struct EmbeddingMetricsResponse {
    /// Number of embed and text-search requests served.
    pub requests: u64,
    /// Number of texts embedded.
    pub texts: u64,
    /// Tokens consumed, as reported by the provider. Absent before a provider reports a count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u64>,
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
}
