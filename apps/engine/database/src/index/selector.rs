//! Build an index from its configuration.

use crate::index::{FlatIndex, HnswIndex, IvfIndex};
use crate::index::{IndexType, VectorIndex};
use piramid_core::config::{
    ExecutionMode, FlatConfig, HnswConfig, IndexConfig, IndexKind, IvfConfig,
};

/// Construct the index the config describes, sized for num_vectors.
pub fn create_index(
    config: &IndexConfig,
    execution: ExecutionMode,
    num_vectors: usize,
) -> Box<dyn VectorIndex> {
    create_index_of_kind(
        config,
        config.select_type(num_vectors),
        execution,
        num_vectors,
    )
}

/// Construct an index of the given family with the parameters config gives that family, sized for
/// num_vectors.
pub(crate) fn create_index_of_kind(
    config: &IndexConfig,
    kind: IndexKind,
    execution: ExecutionMode,
    num_vectors: usize,
) -> Box<dyn VectorIndex> {
    let metric = config.metric();
    let mode = execution;
    let auto = config.auto_config();

    match kind {
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
            // Cluster counts come from ivf_num_clusters when set, else from the collection size.
            _ => {
                let sized = match auto.ivf_num_clusters {
                    Some(num_clusters) => IvfConfig::with_clusters(num_clusters),
                    None => IvfConfig::auto(num_vectors),
                };
                IvfConfig {
                    num_clusters: sized.num_clusters,
                    num_probes: auto.ivf_num_probes.unwrap_or(sized.num_probes),
                    max_iterations: auto.ivf_max_iterations,
                    metric,
                    mode,
                }
            }
        })),
    }
}

/// The configuration family an index type belongs to.
pub(crate) fn kind_of(index_type: IndexType) -> IndexKind {
    match index_type {
        IndexType::Flat => IndexKind::Flat,
        IndexType::Ivf => IndexKind::Ivf,
        IndexType::Hnsw => IndexKind::Hnsw,
    }
}

/// Position of a family in the order an auto index grows through, smallest first.
pub(crate) fn growth_rank(kind: IndexKind) -> u8 {
    match kind {
        IndexKind::Flat => 0,
        IndexKind::Ivf => 1,
        IndexKind::Hnsw => 2,
    }
}
