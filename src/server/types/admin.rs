use serde::Serialize;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
}

#[derive(Serialize)]
pub struct MetricsResponse {
    pub total_collections: usize,
    pub total_vectors: usize,
    pub collections: Vec<CollectionMetrics>,
    pub app_config: crate::config::AppConfig,
    pub wal_stats: Vec<WalStats>,
    pub embedding: EmbeddingMetricsResponse,
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
    pub search_overfetch: Option<usize>,
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

#[derive(Serialize)]
pub struct ConfigStatusResponse {
    pub app_config: crate::config::AppConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reloaded_at: Option<u64>,
}

#[derive(Serialize)]
pub struct ConfigReloadResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reloaded_at: Option<u64>,
    pub app_config: crate::config::AppConfig,
}

#[derive(Serialize)]
pub struct CollectionHealth {
    pub name: String,
    pub loaded: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_checkpoint: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkpoint_age_secs: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wal_size_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_version: Option<u32>,
    pub integrity_ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct ReadyzResponse {
    pub ok: bool,
    pub version: String,
    pub data_dir: String,
    pub total_collections: usize,
    pub loaded_collections: usize,
    pub total_vectors: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk_total_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk_available_bytes: Option<u64>,
    pub collections: Vec<CollectionHealth>,
}
