use serde::{Deserialize, Serialize};

// Storage configuration for the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    // Base path where all collection data will be stored. Each collection will have its own subdirectory under this path.
    pub storage_path: String,
}

impl StorageConfig {
    // new StorageConfig with specified storage path
    pub fn new(path: String) -> Self {
        Self { storage_path: path }
    }
}

impl Default for StorageConfig {
    // default storage configuration with a default storage path
    fn default() -> Self {
        Self::new("./data".to_string())
    }
}
