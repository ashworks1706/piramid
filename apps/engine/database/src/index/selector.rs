//! Build an index from its configuration.

use crate::index::VectorIndex;
use crate::index::{FlatIndex, HnswIndex, IvfIndex};
use piramid_core::config::{
    ExecutionMode, FlatConfig, HnswConfig, IndexConfig, IndexKind, IvfConfig,
};

/// Construct the index the config describes, sized for num_vectors.
pub fn create_index(
    config: &IndexConfig,
    execution: ExecutionMode,
    num_vectors: usize,
) -> Box<dyn VectorIndex> {
    let metric = config.metric();
    let mode = execution;
    let auto = config.auto_config();

    match config.select_type(num_vectors) {
        IndexKind::Flat => Box::new(FlatIndex::new(match config {
            IndexConfig::Flat { params, .. } => FlatConfig { mode, ..*params },
            _ => FlatConfig { metric, mode },
        })),
        IndexKind::Hnsw => Box::new(HnswIndex::new(match config {
            IndexConfig::Hnsw { params, .. } => HnswConfig { mode, ..*params },
            // Graph shape comes from the auto thresholds, with the configured metric and mode.
            _ => HnswConfig {
                metric,
                mode,
                ..HnswConfig::from_m(auto.hnsw_m, auto.hnsw_ef_construction, auto.hnsw_ef_search)
            },
        })),
        IndexKind::Ivf => Box::new(IvfIndex::new(match config {
            IndexConfig::Ivf { params, .. } => IvfConfig { mode, ..*params },
            // Cluster counts come from the collection size, with explicit overrides on top.
            _ => {
                let sized = IvfConfig::auto(num_vectors);
                IvfConfig {
                    num_clusters: auto.ivf_num_clusters.unwrap_or(sized.num_clusters),
                    num_probes: auto.ivf_num_probes.unwrap_or(sized.num_probes),
                    max_iterations: auto.ivf_max_iterations,
                    metric,
                    mode,
                }
            }
        })),
    }
}
