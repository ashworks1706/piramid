//! Index selection configuration.

use serde::{Deserialize, Serialize};

use crate::config::{FlatConfig, HnswConfig, IvfConfig};
use piramid_hardware::compute::Metric;

/// Which index family a configuration resolves to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexKind {
    /// Brute-force scan.
    Flat,
    /// Graph index.
    Hnsw,
    /// Inverted-file index.
    Ivf,
}

/// Thresholds used by [IndexConfig::Auto] to pick a family by collection size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct AutoIndexConfig {
    /// Below this many vectors, the flat index is chosen.
    #[serde(default = "default_flat_max_vectors")]
    pub flat_max_vectors: usize,
    /// Below this many vectors, IVF is chosen; at or above it, HNSW.
    #[serde(default = "default_ivf_max_vectors")]
    pub ivf_max_vectors: usize,
    /// IVF partition count. None sizes it from the collection.
    #[serde(default)]
    pub ivf_num_clusters: Option<usize>,
    /// IVF partitions scanned per query. None sizes it from the collection.
    #[serde(default)]
    pub ivf_num_probes: Option<usize>,
    /// Iteration cap when training IVF centroids.
    #[serde(default = "default_ivf_max_iterations")]
    pub ivf_max_iterations: usize,
    /// HNSW connections per node above layer 0.
    #[serde(default = "default_hnsw_m")]
    pub hnsw_m: usize,
    /// HNSW candidate width while linking a new node.
    #[serde(default = "default_hnsw_ef_construction")]
    pub hnsw_ef_construction: usize,
    /// HNSW candidate width while searching.
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

/// Index configuration for a collection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum IndexConfig {
    /// Pick a family from the collection's size, moving to the next family as the collection
    /// grows past each threshold. A shrinking collection keeps its family.
    Auto {
        /// Distance metric for whichever family is chosen.
        #[serde(default)]
        metric: Metric,
        /// Size thresholds and the parameters of each family.
        #[serde(default)]
        auto: AutoIndexConfig,
    },
    /// Brute-force scan. The parameters are flattened into the enclosing map.
    Flat {
        /// Scan parameters.
        #[serde(flatten)]
        params: FlatConfig,
    },
    /// Graph index.
    Hnsw {
        /// Graph parameters.
        #[serde(flatten)]
        params: HnswConfig,
    },
    /// Inverted-file index.
    Ivf {
        /// Partition parameters.
        #[serde(flatten)]
        params: IvfConfig,
    },
}

impl Default for IndexConfig {
    fn default() -> Self {
        IndexConfig::Auto {
            metric: Metric::Cosine,
            auto: AutoIndexConfig::default(),
        }
    }
}

impl IndexConfig {
    /// Pick a family for a collection of the given vector count.
    pub fn select_type(&self, num_vectors: usize) -> IndexKind {
        match self {
            IndexConfig::Auto { auto, .. } => {
                if num_vectors < auto.flat_max_vectors {
                    IndexKind::Flat
                } else if num_vectors < auto.ivf_max_vectors {
                    IndexKind::Ivf
                } else {
                    IndexKind::Hnsw
                }
            }
            IndexConfig::Flat { .. } => IndexKind::Flat,
            IndexConfig::Hnsw { .. } => IndexKind::Hnsw,
            IndexConfig::Ivf { .. } => IndexKind::Ivf,
        }
    }

    /// The distance metric, whichever variant is configured.
    pub fn metric(&self) -> Metric {
        match self {
            IndexConfig::Auto { metric, .. } => *metric,
            IndexConfig::Flat { params, .. } => params.metric,
            IndexConfig::Hnsw { params, .. } => params.metric,
            IndexConfig::Ivf { params, .. } => params.metric,
        }
    }

    /// Auto-selection thresholds, defaulted for explicit variants.
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

impl IndexConfig {
    /// Check the parameters of whichever variant is configured.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            IndexConfig::Auto { auto, .. } => {
                if auto.flat_max_vectors == 0 || auto.ivf_max_vectors == 0 {
                    return Err("runtime.index.auto: vector thresholds must be > 0".into());
                }
                if auto.flat_max_vectors > auto.ivf_max_vectors {
                    return Err(
                        "runtime.index.auto: flat_max_vectors must be <= ivf_max_vectors".into(),
                    );
                }
                if auto.ivf_max_iterations == 0 || auto.hnsw_m == 0 {
                    return Err(
                        "runtime.index.auto: ivf_max_iterations and hnsw_m must be > 0".into(),
                    );
                }
                if auto.hnsw_ef_construction == 0 || auto.hnsw_ef_search == 0 {
                    return Err("runtime.index.auto: hnsw ef values must be > 0".into());
                }
                Ok(())
            }
            IndexConfig::Flat { .. } => Ok(()),
            IndexConfig::Hnsw { params, .. } => {
                if params.m == 0 || params.m_max == 0 {
                    return Err("runtime.index: hnsw m and m_max must be > 0".into());
                }
                if params.m_max < params.m {
                    return Err("runtime.index: hnsw m_max must be >= m".into());
                }
                if params.ef_construction == 0 || params.ef_search == 0 {
                    return Err(
                        "runtime.index: hnsw ef_construction and ef_search must be > 0".into(),
                    );
                }
                // NaN fails this check as well.
                if !params.ml.is_finite() || params.ml <= 0.0 {
                    return Err("runtime.index: hnsw ml must be a finite number > 0".into());
                }
                Ok(())
            }
            IndexConfig::Ivf { params, .. } => {
                if params.num_clusters == 0 {
                    return Err("runtime.index: ivf num_clusters must be > 0".into());
                }
                if params.num_probes == 0 {
                    return Err("runtime.index: ivf num_probes must be > 0".into());
                }
                if params.num_probes > params.num_clusters {
                    return Err("runtime.index: ivf num_probes must be <= num_clusters".into());
                }
                if params.max_iterations == 0 {
                    return Err("runtime.index: ivf max_iterations must be > 0".into());
                }
                Ok(())
            }
        }
    }
}
