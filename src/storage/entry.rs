// Vector entry - represents a single vector with metadata

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::metadata::Metadata;
use crate::quantization::QuantizedVector;

// A single vector entry stored in the database
// 
// Vectors are stored as quantized int8 for 4x memory efficiency.
// Users work with Vec<f32>, conversion happens automatically.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: Uuid,
    pub vector: QuantizedVector,  // Stored as quantized
    pub text: String,
    #[serde(default)]
    pub metadata: Metadata,
}

impl Document {
    // Create new entry from f32 vector (will be quantized)
    pub fn new(vector: Vec<f32>, text: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            vector: QuantizedVector::from_f32(&vector),
            text,
            metadata: Metadata::new(),
        }
    }

    // Create new entry with metadata (will be quantized)
    pub fn with_metadata(vector: Vec<f32>, text: String, metadata: Metadata) -> Self {
        Self {
            id: Uuid::new_v4(),
            vector: QuantizedVector::from_f32(&vector),
            text,
            metadata,
        }
    }

    // Get the vector as f32 (dequantizes on demand)
    pub fn get_vector(&self) -> Vec<f32> {
        self.vector.to_f32()
    }
}
