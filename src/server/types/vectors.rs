use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::common::DeleteResponse;

#[derive(Deserialize)]
pub struct InsertRequest {
    #[serde(default)]
    pub vector: Option<Vec<f32>>,
    #[serde(default)]
    pub vectors: Option<Vec<Vec<f32>>>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub texts: Option<Vec<String>>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub metadata_list: Vec<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    pub normalize: bool,
}

#[derive(Serialize)]
pub struct InsertResponse {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f32>,
}

#[derive(Serialize)]
pub struct MultiInsertResponse {
    pub ids: Vec<String>,
    pub count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f32>,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum InsertResultsResponse {
    Single(InsertResponse),
    Multi(MultiInsertResponse),
}

#[derive(Serialize)]
pub struct VectorResponse {
    pub id: String,
    pub vector: Vec<f32>,
    pub text: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
pub struct ListVectorsQuery {
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

fn default_limit() -> usize {
    100
}

#[derive(Deserialize)]
pub struct DeleteVectorsRequest {
    pub ids: Vec<String>,
}

#[derive(Serialize)]
pub struct MultiDeleteResponse {
    pub deleted_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f32>,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum DeleteResultsResponse {
    Single(DeleteResponse),
    Multi(MultiDeleteResponse),
}

#[derive(Deserialize)]
pub struct UpsertRequest {
    pub id: Option<String>,
    pub vector: Vec<f32>,
    pub text: String,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub normalize: bool,
}

#[derive(Serialize)]
pub struct UpsertResponse {
    pub id: String,
    pub created: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f32>,
}
