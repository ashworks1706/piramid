// Index selection and factory
// Auto-selects the best index based on collection size and requirements

use serde::{Serialize, Deserialize};
use crate::metrics::Metric;
use crate::config::ExecutionMode;
use crate::config::SearchConfig;

use super::traits::{VectorIndex, IndexType};
use super::{FlatIndex, FlatConfig, HnswIndex, HnswConfig, IvfIndex, IvfConfig};

// Unified index configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum IndexConfig {
    // Auto-select based on size (default)
    Auto { 
        metric: Metric,
        #[serde(default)]
        mode: ExecutionMode,
        #[serde(default)]
        search: SearchConfig,
    },
    // Flat index (brute force)
    Flat { 
        metric: Metric,
        #[serde(default)]
        mode: ExecutionMode,
        #[serde(default)]
        search: SearchConfig,
    },
    // HNSW index
    Hnsw {
        m: usize,
        m_max: usize,
        ef_construction: usize,
        #[serde(default)]
        ef_search: usize,  // New: search-time quality parameter
        ml: f32,
        metric: Metric,
        #[serde(default)]
        mode: ExecutionMode,
        #[serde(default)]
        search: SearchConfig,
    },
    // IVF index
    Ivf {
        num_clusters: usize,
        num_probes: usize,
        max_iterations: usize,
        metric: Metric,
        #[serde(default)]
        mode: ExecutionMode,
        #[serde(default)]
        search: SearchConfig,
    },
}

impl Default for IndexConfig {
    fn default() -> Self {
        IndexConfig::Auto { 
            metric: Metric::Cosine,
            mode: ExecutionMode::default(),
            search: SearchConfig::default(),
        }
    }
}

impl IndexConfig {
    // Select the best index type based on number of vectors
    pub fn select_type(&self, num_vectors: usize) -> IndexType {
        match self {
            IndexConfig::Auto { .. } => {
                if num_vectors < 10_000 {
                    IndexType::Flat
                } else if num_vectors < 100_000 {
                    IndexType::Ivf
                } else {
                    IndexType::Hnsw
                }
            }
            IndexConfig::Flat { .. } => IndexType::Flat,
            IndexConfig::Hnsw { .. } => IndexType::Hnsw,
            IndexConfig::Ivf { .. } => IndexType::Ivf,
        }
    }
    
    // Create an index based on configuration and size
    pub fn create_index(&self, num_vectors: usize) -> Box<dyn VectorIndex> {
        let index_type = self.select_type(num_vectors);
        
        match index_type {
            IndexType::Flat => {
                let (metric, mode) = self.get_metric_and_simd();
                Box::new(FlatIndex::new(FlatConfig { metric, mode }))
            }
            IndexType::Hnsw => {
                let config = match self {
                    IndexConfig::Hnsw { m, m_max, ef_construction, ef_search, ml, metric, mode } => {
                        HnswConfig {
                            m: *m,
                            m_max: *m_max,
                            ef_construction: *ef_construction,
                            ef_search: if *ef_search == 0 { *ef_construction } else { *ef_search },
                            ml: *ml,
                            metric: *metric,
                            mode: *mode,
                        }
                    }
                    _ => {
                        let (metric, mode) = self.get_metric_and_simd();
                        HnswConfig {
                            m: 16,
                            m_max: 32,
                            ef_construction: 200,
                            ef_search: 200,
                            ml: 1.0 / (16.0_f32).ln(),
                            metric,
                            mode,
                        }
                    }
                };
                Box::new(HnswIndex::new(config))
            }
            IndexType::Ivf => {
                let config = match self {
                    IndexConfig::Ivf { num_clusters, num_probes, max_iterations, metric, mode } => {
                        IvfConfig {
                            num_clusters: *num_clusters,
                            num_probes: *num_probes,
                            max_iterations: *max_iterations,
                            metric: *metric,
                            mode: *mode,
                        }
                    }
                    _ => {
                        let (metric, mode) = self.get_metric_and_simd();
                        let mut config = IvfConfig::auto(num_vectors);
                        config.metric = metric;
                        config.mode = mode;
                        config
                    }
                };
                Box::new(IvfIndex::new(config))
            }
        }
    }
    
    #[allow(dead_code)]
    fn get_metric(&self) -> Metric {
        match self {
            IndexConfig::Auto { metric, .. } => *metric,
            IndexConfig::Flat { metric, .. } => *metric,
            IndexConfig::Hnsw { metric, .. } => *metric,
            IndexConfig::Ivf { metric, .. } => *metric,
        }
    }
    
    fn get_metric_and_simd(&self) -> (Metric, ExecutionMode) {
        match self {
            IndexConfig::Auto { metric, mode, .. } => (*metric, *mode),
            IndexConfig::Flat { metric, mode, .. } => (*metric, *mode),
            IndexConfig::Hnsw { metric, mode, .. } => (*metric, *mode),
            IndexConfig::Ivf { metric, mode, .. } => (*metric, *mode),
        }
    }

    pub fn search_config(&self) -> SearchConfig {
        match self {
            IndexConfig::Auto { search, .. } => search.clone(),
            IndexConfig::Flat { search, .. } => search.clone(),
            IndexConfig::Hnsw { search, .. } => search.clone(),
            IndexConfig::Ivf { search, .. } => search.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_selection() {
        let config = IndexConfig::default();
        
        assert_eq!(config.select_type(1_000), IndexType::Flat);
        assert_eq!(config.select_type(50_000), IndexType::Ivf);
        assert_eq!(config.select_type(500_000), IndexType::Hnsw);
    }
    
    #[test]
    fn test_forced_index_type() {
        let flat_config = IndexConfig::Flat { 
            metric: Metric::Cosine,
            mode: ExecutionMode::default(),
        };
        assert_eq!(flat_config.select_type(1_000_000), IndexType::Flat);
        
        let hnsw_config = IndexConfig::Hnsw {
            m: 16,
            m_max: 32,
            ef_construction: 200,
            ef_search: 200,
            ml: 1.0 / 16.0_f32.ln(),
            metric: Metric::Cosine,
            mode: ExecutionMode::default(),
        };
        assert_eq!(hnsw_config.select_type(100), IndexType::Hnsw);
    }
}
