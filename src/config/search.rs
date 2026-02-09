// Search configuration
// Allows users to control the recall/speed tradeoff at search time

use serde::{Deserialize, Serialize};

/// Search configuration parameters
/// Different index types use different parameters:
/// - HNSW: uses ef (candidates explored during search)
/// - IVF: uses nprobe (number of clusters to search)
/// - Flat: always exhaustive (ignores these settings)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SearchConfig {
    /// HNSW: Number of candidates to explore (higher = better recall, slower)
    /// Default: uses ef_search from config, or ef_construction if not set
    pub ef: Option<usize>,
    
    /// IVF: Number of clusters to probe (higher = better recall, slower)
    /// Default: uses num_probes from config
    pub nprobe: Option<usize>,
}

impl Default for SearchConfig {
    fn default() -> Self {
        SearchConfig {
            ef: None,      // Use index config default
            nprobe: None,  // Use index config default
        }
    }
}

impl SearchConfig {
    /// High quality search (better recall, slower)
    pub fn high() -> Self {
        SearchConfig {
            ef: Some(400),
            nprobe: Some(20),
        }
    }
    
    /// Balanced search (default)
    pub fn balanced() -> Self {
        SearchConfig::default()
    }
    
    /// Fast search (lower recall, faster)
    pub fn fast() -> Self {
        SearchConfig {
            ef: Some(50),
            nprobe: Some(1),
        }
    }
}
