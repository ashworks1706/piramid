use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub fn default_k() -> usize {
    10
}

#[derive(Deserialize)]
pub struct SearchRequest {
    #[serde(default)]
    pub vector: Option<Vec<f32>>,
    #[serde(default)]
    pub vectors: Option<Vec<Vec<f32>>>,
    #[serde(default = "default_k")]
    pub k: usize,
    #[serde(default)]
    pub metric: Option<String>,
    #[serde(default)]
    pub ef: Option<usize>,
    #[serde(default)]
    pub nprobe: Option<usize>,
    #[serde(default)]
    pub overfetch: Option<usize>,
    #[serde(default)]
    pub preset: Option<String>,
}

#[derive(Serialize)]
pub struct HitResponse {
    pub id: String,
    pub score: f32,
    pub text: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Serialize)]
pub struct SearchResponse {
    pub results: Vec<HitResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f32>,
}

#[derive(Serialize)]
pub struct MultiSearchResponse {
    pub results: Vec<Vec<HitResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f32>,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum SearchResultsResponse {
    Single(SearchResponse),
    Multi(MultiSearchResponse),
}
