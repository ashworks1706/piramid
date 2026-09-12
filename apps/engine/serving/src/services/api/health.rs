//! Liveness and readiness responses.

use serde::Serialize;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
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
    /// Whether the collection opened cleanly. Absent for a collection not yet opened, which has
    /// not been checked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integrity_ok: Option<bool>,
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
