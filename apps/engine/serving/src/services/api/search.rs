//! Vector search request and response shapes.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Number of hits a search returns when the request omits k.
pub fn default_k() -> usize {
    10
}

/// Search a collection with one or more query vectors.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchRequest {
    /// Query vectors, searched as one batch. Must not be empty.
    pub vectors: Vec<Vec<f32>>,
    /// Maximum number of hits per query. 10 when omitted.
    #[serde(default = "default_k")]
    pub k: usize,
    /// Similarity metric: cosine, euclidean or dot. The metric of the collection when omitted.
    #[serde(default)]
    pub metric: Option<String>,
    /// Metadata predicate, mapping a field name to an operator and value.
    #[serde(default)]
    pub filter: Option<HashMap<String, HashMap<String, serde_json::Value>>>,
}

/// One search hit.
#[derive(Serialize)]
pub struct HitResponse {
    /// Id of the matched document.
    pub id: String,
    /// Similarity to the query under the requested metric; higher is closer.
    pub score: f32,
    /// Text stored with the document.
    pub text: String,
    /// Metadata stored with the document.
    pub metadata: HashMap<String, serde_json::Value>,
}

/// One result list per query vector, in request order.
#[derive(Serialize)]
pub struct SearchResponse {
    /// Hits for each query, highest score first.
    pub results: Vec<Vec<HitResponse>>,
    /// Duration of the search itself, excluding embedding and lock wait, in whole milliseconds.
    pub latency_ms: f32,
}
