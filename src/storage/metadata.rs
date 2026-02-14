// Collection metadata tracking (created_at, updated_at, dimensions)

use serde::{Serialize, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionMetadata {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub name: String,
    pub created_at: u64,      // Unix timestamp (seconds)
    pub updated_at: u64,      // Unix timestamp (seconds)
    pub dimensions: Option<usize>,  // Expected vector dimensions (None = auto-detect)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_new_metadata() {
        let meta = CollectionMetadata::new("test".to_string());
        assert_eq!(meta.name, "test");
        assert!(meta.created_at > 0);
        assert_eq!(meta.created_at, meta.updated_at);
        assert_eq!(meta.dimensions, None);
        assert_eq!(meta.vector_count, 0);
    }

    #[test]
    fn test_with_dimensions() {
        let meta = CollectionMetadata::with_dimensions("test".to_string(), 768);
        assert_eq!(meta.dimensions, Some(768));
    }

    #[test]
    fn test_touch() {
        let mut meta = CollectionMetadata::new("test".to_string());
        let original_time = meta.updated_at;
        
        thread::sleep(Duration::from_millis(10));
        meta.touch();
        
        assert!(meta.updated_at >= original_time);
        assert_eq!(meta.created_at, original_time); // created_at doesn't change
    }

    #[test]
    fn test_set_dimensions() {
        let mut meta = CollectionMetadata::new("test".to_string());
        meta.set_dimensions(512);
        assert_eq!(meta.dimensions, Some(512));
        
        // Should not change if already set
        meta.set_dimensions(1024);
        assert_eq!(meta.dimensions, Some(512));
    }

    #[test]
    fn test_update_vector_count() {
        let mut meta = CollectionMetadata::new("test".to_string());
        let original_time = meta.updated_at;
        
        thread::sleep(Duration::from_millis(10));
        meta.update_vector_count(100);
        
        assert_eq!(meta.vector_count, 100);
        assert!(meta.updated_at >= original_time);
    }
}
