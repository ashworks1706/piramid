//! Memory limits and mmap settings.

use serde::{Deserialize, Serialize};

/// Memory ceiling for a collection and how its data file is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct MemoryConfig {
    /// Byte ceiling per collection. None is unbounded.
    pub max_memory_per_collection: Option<usize>,

    /// Size the data file is first mapped at. It grows from here.
    pub initial_mmap_size: usize,

    /// Map the data file. With this off, records go through ordinary file reads.
    pub use_mmap: bool,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        MemoryConfig {
            max_memory_per_collection: None,
            initial_mmap_size: 1024 * 1024,
            use_mmap: true,
        }
    }
}

impl MemoryConfig {
    /// Unbounded memory with the data file read through ordinary file reads.
    pub fn no_mmap() -> Self {
        MemoryConfig {
            max_memory_per_collection: None,
            initial_mmap_size: 0,
            use_mmap: false,
        }
    }
}
