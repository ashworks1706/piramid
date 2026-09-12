//! Document insert, read, list, delete and upsert request and response shapes.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Insert one or more documents. Always a list, even for one document.
#[derive(Deserialize)]
pub struct InsertRequest {
    /// Vector of each document. Must not be empty.
    pub vectors: Vec<Vec<f32>>,
    /// Text of each document, the same length as the vector list.
    pub texts: Vec<String>,
    /// One map per vector. Empty means no metadata on any of them; otherwise it must be the same
    /// length as the vector list.
    #[serde(default)]
    pub metadata: Vec<HashMap<String, serde_json::Value>>,
    /// Whether each vector is scaled to unit length before storing. False when omitted.
    #[serde(default)]
    pub normalize: bool,
}

/// Documents stored by an insert.
#[derive(Serialize)]
pub struct InsertResponse {
    /// Ids assigned to the stored documents, in request order.
    pub ids: Vec<String>,
    /// Number of documents stored.
    pub count: usize,
    /// Duration of the write itself, excluding validation and lock wait, in whole milliseconds.
    pub latency_ms: f32,
}

/// One stored document.
#[derive(Serialize)]
pub struct VectorResponse {
    /// Document id.
    pub id: String,
    /// Stored vector.
    pub vector: Vec<f32>,
    /// Stored text.
    pub text: String,
    /// Stored metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Query parameters for paging through the documents of a collection.
#[derive(Deserialize)]
pub struct ListVectorsQuery {
    /// Maximum number of documents returned. 100 when omitted.
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Number of documents skipped before the first one returned. 0 when omitted.
    #[serde(default)]
    pub offset: usize,
}

fn default_limit() -> usize {
    100
}

/// Delete several documents by id.
#[derive(Deserialize)]
pub struct DeleteVectorsRequest {
    /// Ids of the documents to delete, each a UUID.
    pub ids: Vec<String>,
}

/// Result of a delete.
#[derive(Serialize)]
pub struct DeleteResponse {
    /// Number of documents that existed and were deleted.
    pub deleted_count: usize,
    /// Duration of the delete itself, excluding lock wait, in whole milliseconds.
    pub latency_ms: f32,
}

/// Insert a document, replacing any existing one with the same id.
#[derive(Deserialize)]
pub struct UpsertRequest {
    /// Id of the document, a UUID. A new id is generated when omitted.
    pub id: Option<String>,
    /// Vector of the document.
    pub vector: Vec<f32>,
    /// Text of the document.
    pub text: String,
    /// Metadata of the document. Empty when omitted.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
    /// Whether the vector is scaled to unit length before storing. False when omitted.
    #[serde(default)]
    pub normalize: bool,
}

/// Result of an upsert.
#[derive(Serialize)]
pub struct UpsertResponse {
    /// Id of the stored document.
    pub id: String,
    /// True when no document with that id existed before.
    pub created: bool,
    /// Duration of the write itself, excluding validation and lock wait, in whole milliseconds.
    pub latency_ms: f32,
}
