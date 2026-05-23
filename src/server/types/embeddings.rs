use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::search::default_k;

#[derive(Deserialize)]
pub struct EmbedRequest {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub texts: Option<Vec<String>>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub metadata_list: Vec<HashMap<String, serde_json::Value>>,
}

#[derive(Serialize)]
pub struct EmbedResponse {
    pub id: String,
    pub embedding: Vec<f32>,
    pub tokens: Option<u32>,
}

#[derive(Serialize)]
pub struct MultiEmbedResponse {
    pub ids: Vec<String>,
    pub embeddings: Vec<Vec<f32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u32>,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum EmbedResultsResponse {
    Single(EmbedResponse),
    Multi(MultiEmbedResponse),
}

#[derive(Deserialize)]
pub struct TextSearchRequest {
    pub query: String,
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
