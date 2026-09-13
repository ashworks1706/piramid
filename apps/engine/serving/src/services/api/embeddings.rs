//! Embed and text-search request and response shapes.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::search::default_k;

/// Embed one or more texts and store them.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbedRequest {
    /// Texts to embed, one document per text. Must not be empty.
    pub texts: Vec<String>,
    /// One map per text; empty means no metadata, otherwise must match the text list length.
    #[serde(default)]
    pub metadata: Vec<HashMap<String, serde_json::Value>>,
}

/// Documents stored from embedded texts.
#[derive(Serialize)]
pub struct EmbedResponse {
    /// Ids of the stored documents, in request order.
    pub ids: Vec<String>,
    /// Embedding of each text, in request order.
    pub embeddings: Vec<Vec<f32>>,
    /// Tokens consumed across all texts; null if any text went uncounted.
    pub total_tokens: Option<u32>,
}

/// Embed the query text and search with the resulting vector.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextSearchRequest {
    /// Text to embed as the query.
    pub query: String,
    /// Maximum number of hits returned. 10 when omitted.
    #[serde(default = "default_k")]
    pub k: usize,
    /// Similarity metric: cosine, euclidean or dot. The metric of the collection when omitted.
    #[serde(default)]
    pub metric: Option<String>,
    /// Metadata predicate, mapping a field name to an operator and value.
    #[serde(default)]
    pub filter: Option<HashMap<String, HashMap<String, serde_json::Value>>>,
}
