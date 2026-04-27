// Search configuration

use serde::{Deserialize, Serialize};

// index types use different parameters:
// - HNSW uses ef (candidates explored during search)
// - IVF uses nprobe (number of clusters to search)
// - Flat always exhaustive (ignores these settings)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SearchConfig {
    //  uses ef_search from config, or ef_construction if not set
    pub ef: Option<usize>,
    
    //  uses num_probes from config
    pub nprobe: Option<usize>,

    // How many extra candidates to pull when a filter is present multiplier of k
    #[serde(default = "default_filter_overfetch")]
    pub filter_overfetch: usize,
}

impl Default for SearchConfig {
    fn default() -> Self {
        SearchConfig {
            ef: None,      
            nprobe: None,  
            filter_overfetch: default_filter_overfetch(),
        }
    }
}

impl SearchConfig {
    // better recall, slower
    pub fn high() -> Self {
        SearchConfig {
            ef: Some(400),
            nprobe: Some(20),
            filter_overfetch: default_filter_overfetch(),
        }
    }
    
    // default
    pub fn balanced() -> Self {
        SearchConfig::default()
    }
    
    // lower recall, faster
    pub fn fast() -> Self {
        SearchConfig {
            ef: Some(50),
            nprobe: Some(1),
            filter_overfetch: default_filter_overfetch(),
        }
    }
}

fn default_filter_overfetch() -> usize { 10 }
