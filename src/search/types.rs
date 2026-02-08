// Search result type - what you get back from a search query

use uuid::Uuid;
use crate::metadata::Metadata;

// A search result containing the vector entry plus its similarity score
#[derive(Debug, Clone)]
pub struct Hit {
    pub id: Uuid,
    pub score: f32,      
    pub text: String,
    pub vector: Vec<f32>,
    pub metadata: Metadata,
}

impl Hit {
    pub fn new(id: Uuid, score: f32, text: String, vector: Vec<f32>, metadata: Metadata) -> Self {
        Self { id, score, text, vector, metadata }
    }
}
