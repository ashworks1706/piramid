// Memory management configuration

use serde::{Deserialize, Serialize};

// Memory limit configuration
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MemoryConfig {
    // Maximum memory per collection in bytes (None = unlimited)
    pub max_memory_per_collection: Option<usize>,
    
    // Initial mmap size in bytes
    pub initial_mmap_size: usize,
    
    // Enable memory-mapped files
    pub use_mmap: bool,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        MemoryConfig {
            max_memory_per_collection: None,  // Unlimited
            initial_mmap_size: 1024 * 1024,   // 1MB
            use_mmap: true,
        }
    }
}

impl MemoryConfig {
    // Set maximum memory per collection in MB
    pub fn with_limit_mb(limit_mb: usize) -> Self {
        MemoryConfig {
            max_memory_per_collection: Some(limit_mb * 1024 * 1024),
            initial_mmap_size: 1024 * 1024,
            use_mmap: true,
        }
    }
    
    // Set initial mmap size in MB
    pub fn with_mmap_size_mb(size_mb: usize) -> Self {
        MemoryConfig {
            max_memory_per_collection: None,
            initial_mmap_size: size_mb * 1024 * 1024,
            use_mmap: true,
        }
    }
    
    // Disable memory-mapped files (use regular heap allocation)
    pub fn no_mmap() -> Self {
        MemoryConfig {
            max_memory_per_collection: None,
            initial_mmap_size: 0,
            use_mmap: false,
        }
    }
}
