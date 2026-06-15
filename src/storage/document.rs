// Vector entry - represents a single vector with metadata

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::metadata::Metadata;
use crate::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: Uuid,
    pub vector: Vec<f32>,
    pub text: String,
    #[serde(default)]
    pub metadata: Metadata,
}

impl Document {
    pub fn new(vector: Vec<f32>, text: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            vector,
            text,
            metadata: Metadata::new(),
        }
    }

    pub fn with_metadata(vector: Vec<f32>, text: String, metadata: Metadata) -> Self {
        Self {
            id: Uuid::new_v4(),
            vector,
            text,
            metadata,
        }
    }

    pub fn get_vector(&self) -> Vec<f32> {
        self.vector.clone()
    }

    pub fn try_get_vector(&self) -> Result<Vec<f32>> {
        Ok(self.get_vector())
    }
}
