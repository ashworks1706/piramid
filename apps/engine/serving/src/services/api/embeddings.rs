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
    /// One map per text. Empty means no metadata on any of them; otherwise it must be the same
    /// length as the text list.
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
    /// Tokens consumed across all texts, as reported by the provider. Null when the provider
    /// reports no count for any one of the texts.
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
    /// Similarity metric: cosine, euclidean or dot. The metric the collection is indexed by when
    /// omitted.
    #[serde(default)]
    pub metric: Option<String>,
    /// Metadata predicate, mapping a field name to an operator and value.
    #[serde(default)]
    pub filter: Option<HashMap<String, HashMap<String, serde_json::Value>>>,
    /// HNSW candidate-list width. The collection default when omitted.
    #[serde(default)]
    pub ef: Option<usize>,
    /// IVF partitions to scan. The collection default when omitted.
    #[serde(default)]
    pub nprobe: Option<usize>,
    /// Multiplier applied to k when a filter is present. The collection default when omitted.
    #[serde(default)]
    pub filter_overfetch: Option<usize>,
}
