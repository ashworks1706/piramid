use serde::{Deserialize, Serialize};

// Storage configuration for the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    // Base path where all collection data will be stored. Each collection will have its own subdirectory under this path.
    pub storage_path: String,
}

impl StorageConfig {
    // Create a new StorageConfig with the specified storage path. This allows users to specify where they want their collection data to be stored on disk.
    pub fn new(path: String) -> Self {
        Self { storage_path: path }
    }
}

impl Default for StorageConfig {
    // Provide a default storage configuration with a default storage path. This allows users to use the default configuration without needing to specify a storage path, while still providing a sensible default location for storing collection data.
    fn default() -> Self {
        Self::new("./data".to_string())
    }
}
