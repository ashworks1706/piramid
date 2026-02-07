// Index selection and factory
// Auto-selects the best index based on collection size and requirements

use serde::{Serialize, Deserialize};
use crate::metrics::Metric;

use super::traits::{VectorIndex, IndexType};
use super::{FlatIndex, FlatConfig, HnswIndex, HnswConfig, IvfIndex, IvfConfig};

// Unified index configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum IndexConfig {
    // Auto-select based on size (default)
    Auto { metric: Metric },
    // Flat index (brute force)
    Flat { metric: Metric },
    // HNSW index
    Hnsw {
        m: usize,
        m_max: usize,
        ef_construction: usize,
        ml: f32,
        metric: Metric,
    },
    // IVF index
    Ivf {
        num_clusters: usize,
        num_probes: usize,
        max_iterations: usize,
        metric: Metric,
    },
}

impl Default for IndexConfig {
    fn default() -> Self {
        IndexConfig::Auto { metric: Metric::Cosine }
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
                let metric = self.get_metric();
                Box::new(FlatIndex::new(FlatConfig { metric }))
            }
            IndexType::Hnsw => {
                let config = match self {
                    IndexConfig::Hnsw { m, m_max, ef_construction, ml, metric } => {
                        HnswConfig {
                            m: *m,
                            m_max: *m_max,
                            ef_construction: *ef_construction,
                            ml: *ml,
                            metric: *metric,
                        }
                    }
                    _ => {
                        let metric = self.get_metric();
                        HnswConfig {
                            m: 16,
                            m_max: 32,
                            ef_construction: 200,
                            ml: 1.0 / (16.0_f32).ln(),
                            metric,
                        }
                    }
                };
                Box::new(HnswIndex::new(config))
            }
            IndexType::Ivf => {
                let config = match self {
                    IndexConfig::Ivf { num_clusters, num_probes, max_iterations, metric } => {
                        IvfConfig {
                            num_clusters: *num_clusters,
                            num_probes: *num_probes,
                            max_iterations: *max_iterations,
                            metric: *metric,
                        }
                    }
                    _ => {
                        let metric = self.get_metric();
                        let mut config = IvfConfig::auto(num_vectors);
                        config.metric = metric;
                        config
                    }
                };
                Box::new(IvfIndex::new(config))
            }
        }
    }
    
    fn get_metric(&self) -> Metric {
        match self {
            IndexConfig::Auto { metric } => *metric,
            IndexConfig::Flat { metric } => *metric,
            IndexConfig::Hnsw { metric, .. } => *metric,
            IndexConfig::Ivf { metric, .. } => *metric,
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
        let flat_config = IndexConfig::Flat { metric: Metric::Cosine };
        assert_eq!(flat_config.select_type(1_000_000), IndexType::Flat);
        
        let hnsw_config = IndexConfig::Hnsw {
            m: 16,
            m_max: 32,
            ef_construction: 200,
            ml: 1.0 / 16.0_f32.ln(),
            metric: Metric::Cosine,
        };
        assert_eq!(hnsw_config.select_type(100), IndexType::Hnsw);
    }
}
