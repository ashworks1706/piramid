// Collection metadata tracking (created_at, updated_at, dimensions)

use serde::{Serialize, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionMetadata {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub name: String,
    pub created_at: u64,      // Unix timestamp (seconds)
    pub updated_at: u64,      
    pub dimensions: Option<usize>,  
    pub vector_count: usize,
}

pub const SCHEMA_VERSION: u32 = 1;

fn default_schema_version() -> u32 {
    SCHEMA_VERSION
}

impl CollectionMetadata {
    pub fn new(name: String) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self {
            schema_version: SCHEMA_VERSION,
            name,
            created_at: now,
            updated_at: now,
            dimensions: None,
            vector_count: 0,
        }
    }
    
    pub fn with_dimensions(name: String, dimensions: usize) -> Self {
        let mut meta = Self::new(name);
        meta.dimensions = Some(dimensions);
        meta
    }
    
    pub fn touch(&mut self) {
        self.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
    
    pub fn set_dimensions(&mut self, dimensions: usize) {
        if self.dimensions.is_none() {
            self.dimensions = Some(dimensions);
        }
    }
    
    pub fn update_vector_count(&mut self, count: usize) {
        self.vector_count = count;
        self.touch();
    }
}
