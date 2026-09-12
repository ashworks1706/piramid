//! The record a collection stores, and a scored one.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::metadata::Metadata;

/// A stored record: a vector, the text it was made from, and its metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Identifier, unique within a collection.
    pub id: Uuid,
    /// The embedding searched against.
    pub vector: Vec<f32>,
    /// Source text the vector represents.
    pub text: String,
    /// Key-value fields filters are evaluated against.
    #[serde(default)]
    pub metadata: Metadata,
}

impl Document {
    /// A document with a fresh random id and empty metadata.
    pub fn new(vector: Vec<f32>, text: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            vector,
            text,
            metadata: Metadata::new(),
        }
    }

    /// A document with a fresh random id and the given metadata.
    pub fn with_metadata(vector: Vec<f32>, text: String, metadata: Metadata) -> Self {
        Self {
            id: Uuid::new_v4(),
            vector,
            text,
            metadata,
        }
    }

    /// The stored vector.
    pub fn vector(&self) -> &[f32] {
        &self.vector
    }
}

/// A search result: a stored document and how well it matched.
///
/// Holds the [Document] itself rather than restating its fields.
#[derive(Debug, Clone)]
pub struct Hit {
    /// Similarity, normalised so higher is closer.
    pub score: f32,
    /// The document that matched.
    pub document: Document,
}
