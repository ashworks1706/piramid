//! What you get back from a search query

use uuid::Uuid;
use crate::metadata::Metadata;

/// A search result - the vector entry plus its similarity score
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: Uuid,
    pub score: f32,       // higher = more similar
    pub text: String,
    pub vector: Vec<f32>,
    pub metadata: Metadata,
}

impl SearchResult {
    pub fn new(id: Uuid, score: f32, text: String, vector: Vec<f32>, metadata: Metadata) -> Self {
        Self { id, score, text, vector, metadata }
    }
}
