use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct CollectionInfo {
    pub name: String,
    pub count: usize,
    pub created_at: Option<u64>,
    pub updated_at: Option<u64>,
    pub dimensions: Option<usize>,
}

#[derive(Serialize)]
pub struct CollectionsResponse {
    pub collections: Vec<CollectionInfo>,
}

#[derive(Deserialize)]
pub struct CreateCollectionRequest {
    pub name: String,
}

#[derive(Deserialize)]
pub struct DuplicateRequest {
    #[serde(default)]
    pub metric: Option<String>,
    #[serde(default = "default_dup_threshold")]
    pub threshold: f32,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub k: Option<usize>,
    #[serde(default)]
    pub ef: Option<usize>,
    #[serde(default)]
    pub nprobe: Option<usize>,
}

fn default_dup_threshold() -> f32 {
    0.95
}

#[derive(Serialize)]
pub struct DuplicatePair {
    pub id_a: String,
    pub id_b: String,
    pub score: f32,
}

#[derive(Serialize)]
pub struct DuplicateResponse {
    pub pairs: Vec<DuplicatePair>,
}

#[derive(Serialize)]
pub struct IndexStatsResponse {
    pub index_type: String,
    pub total_vectors: usize,
    pub memory_usage_bytes: usize,
    pub details: serde_json::Value,
}

#[derive(Serialize)]
pub struct RebuildIndexResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f32>,
}

#[derive(Serialize)]
pub struct RebuildIndexStatusResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elapsed_ms: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
