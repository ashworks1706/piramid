//  a unified configuration interface for different types of vector indices (Flat, HNSW, IVF).
use crate::config::ExecutionMode;
use crate::config::SearchConfig;
use crate::metrics::Metric;
use serde::{Deserialize, Serialize};

use super::traits::{IndexType, VectorIndex};
use super::{FlatConfig, FlatIndex, HnswConfig, HnswIndex, IvfConfig, IvfIndex};

// Unified index configuration
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AutoIndexConfig {
    #[serde(default = "default_flat_max_vectors")]
    pub flat_max_vectors: usize,
    #[serde(default = "default_ivf_max_vectors")]
    pub ivf_max_vectors: usize,
    #[serde(default)]
    pub ivf_num_clusters: Option<usize>,
    #[serde(default)]
    pub ivf_num_probes: Option<usize>,
    #[serde(default = "default_ivf_max_iterations")]
    pub ivf_max_iterations: usize,
    #[serde(default = "default_hnsw_m")]
    pub hnsw_m: usize,
    #[serde(default = "default_hnsw_ef_construction")]
    pub hnsw_ef_construction: usize,
    #[serde(default = "default_hnsw_ef_search")]
    pub hnsw_ef_search: usize,
}

impl Default for AutoIndexConfig {
    fn default() -> Self {
        Self {
            flat_max_vectors: default_flat_max_vectors(),
            ivf_max_vectors: default_ivf_max_vectors(),
            ivf_num_clusters: None,
            ivf_num_probes: None,
            ivf_max_iterations: default_ivf_max_iterations(),
            hnsw_m: default_hnsw_m(),
            hnsw_ef_construction: default_hnsw_ef_construction(),
            hnsw_ef_search: default_hnsw_ef_search(),
        }
    }
}

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
        #[serde(default)]
        auto: AutoIndexConfig,
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
        ef_search: usize,
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
            auto: AutoIndexConfig::default(),
        }
    }
}

// Enum to represent the selected index type after auto-selection
impl IndexConfig {
    // Select the best index type based on number of vectors
    pub fn select_type(&self, num_vectors: usize) -> IndexType {
        match self {
            IndexConfig::Auto { auto, .. } => {
                if num_vectors < auto.flat_max_vectors {
                    IndexType::Flat
                } else if num_vectors < auto.ivf_max_vectors {
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
                // we use the metric and mode from the config, but the rest of the parameters are not needed for a flat index.
                let (metric, mode) = self.get_metric_and_simd();
                Box::new(FlatIndex::new(FlatConfig { metric, mode }))
            }
            IndexType::Hnsw => {
                let config = match self {
                    IndexConfig::Hnsw {
                        m,
                        m_max,
                        ef_construction,
                        ef_search,
                        ml,
                        metric,
                        mode,
                        ..
                    } => HnswConfig {
                        m: *m,
                        m_max: *m_max,
                        ef_construction: *ef_construction,
                        ef_search: if *ef_search == 0 {
                            *ef_construction
                        } else {
                            *ef_search
                        },
                        ml: *ml,
                        metric: *metric,
                        mode: *mode,
                    },
                    _ => {
                        //  we use default HNSW parameters but apply the metric and mode from the config. The ef_search parameter defaults to the same value as ef_construction if not explicitly set
                        let (metric, mode) = self.get_metric_and_simd();
                        let auto = self.auto_config();
                        let m = auto.hnsw_m;
                        HnswConfig {
                            m,
                            m_max: m * 2,
                            ef_construction: auto.hnsw_ef_construction,
                            ef_search: auto.hnsw_ef_search,
                            ml: 1.0 / (m as f32).ln(),
                            metric,
                            mode,
                        }
                    }
                };
                Box::new(HnswIndex::new(config))
            }
            IndexType::Ivf => {
                let config = match self {
                    IndexConfig::Ivf {
                        num_clusters,
                        num_probes,
                        max_iterations,
                        metric,
                        mode,
                        ..
                    } => IvfConfig {
                        num_clusters: *num_clusters,
                        num_probes: *num_probes,
                        max_iterations: *max_iterations,
                        metric: *metric,
                        mode: *mode,
                    },
                    _ => {
                        // we use the auto-configure method to determine the number of clusters and probes based on the number of vectors, while applying the metric and mode from the config. configured dynamically based on the dataset size while still respecting user preferences for the distance metric and execution mode.
                        let (metric, mode) = self.get_metric_and_simd();
                        let auto = self.auto_config();
                        let mut config = IvfConfig::auto(num_vectors);
                        if let Some(num_clusters) = auto.ivf_num_clusters {
                            config.num_clusters = num_clusters;
                        }
                        if let Some(num_probes) = auto.ivf_num_probes {
                            config.num_probes = num_probes;
                        }
                        config.max_iterations = auto.ivf_max_iterations;
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
        self.get_metric_and_mode()
    }

    pub fn get_metric_and_mode(&self) -> (Metric, ExecutionMode) {
        match self {
            IndexConfig::Auto { metric, mode, .. } => (*metric, *mode),
            IndexConfig::Flat { metric, mode, .. } => (*metric, *mode),
            IndexConfig::Hnsw { metric, mode, .. } => (*metric, *mode),
            IndexConfig::Ivf { metric, mode, .. } => (*metric, *mode),
        }
    }

    pub fn search_config(&self) -> SearchConfig {
        match self {
            IndexConfig::Auto { search, .. } => *search,
            IndexConfig::Flat { search, .. } => *search,
            IndexConfig::Hnsw { search, .. } => *search,
            IndexConfig::Ivf { search, .. } => *search,
        }
    }

    pub fn auto_config(&self) -> AutoIndexConfig {
        match self {
            IndexConfig::Auto { auto, .. } => *auto,
            _ => AutoIndexConfig::default(),
        }
    }
}

fn default_flat_max_vectors() -> usize {
    10_000
}

fn default_ivf_max_vectors() -> usize {
    100_000
}

fn default_ivf_max_iterations() -> usize {
    10
}

fn default_hnsw_m() -> usize {
    16
}

fn default_hnsw_ef_construction() -> usize {
    200
}

fn default_hnsw_ef_search() -> usize {
    200
}
