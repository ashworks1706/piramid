use serde::{Deserialize, Serialize};

// Storage configuration for the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub storage_path: String,
}

impl StorageConfig {
    pub fn new(path: String) -> Self {
        Self { storage_path: path }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self::new("./data".to_string())
    }
}
