// Cache configuration for embeddings

use serde::{Deserialize, Serialize};

// Cache configuration
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CacheConfig {
    // Enable caching
    pub enabled: bool,
    
    // Maximum number of cached items
    pub max_size: usize,
    
    // Time-to-live in seconds (None = no expiration)
    pub ttl_seconds: Option<u64>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        CacheConfig {
            enabled: true,
            max_size: 10_000,
            ttl_seconds: None,
        }
    }
}

impl CacheConfig {
    // Disable caching
    pub fn disabled() -> Self {
        CacheConfig {
            enabled: false,
            max_size: 0,
            ttl_seconds: None,
        }
    }
    
    // Set cache size
    pub fn with_size(size: usize) -> Self {
        CacheConfig {
            enabled: true,
            max_size: size,
            ttl_seconds: None,
        }
    }
    
    // Set cache size and TTL
    pub fn with_size_and_ttl(size: usize, ttl_seconds: u64) -> Self {
        CacheConfig {
            enabled: true,
            max_size: size,
            ttl_seconds: Some(ttl_seconds),
        }
    }
}
