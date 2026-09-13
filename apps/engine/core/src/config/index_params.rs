//! Per-family index parameters.
//!
//! One struct per family, carrying the same values an [IndexConfig](super::IndexConfig) variant
//! does. The index crate builds directly from them.

use serde::{Deserialize, Serialize};

use piramid_hardware::compute::{ExecutionMode, Metric};

/// Brute-force scan parameters.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlatConfig {
    /// What to measure.
    pub metric: Metric,

    /// Which strategy runs the math. Set from runtime.execution when the index is built.
    #[serde(skip)]
    pub mode: ExecutionMode,
}

impl Default for FlatConfig {
    fn default() -> Self {
        Self {
            metric: Metric::Cosine,
            mode: ExecutionMode::default(),
        }
    }
}

/// Graph index parameters.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HnswConfig {
    /// Max connections per node above layer 0.
    pub m: usize,
    /// Max connections at layer 0, conventionally twice m.
    pub m_max: usize,
    /// Candidate list width while linking a new node.
    pub ef_construction: usize,
    /// Candidate list width while searching. Higher is better recall and slower.
    pub ef_search: usize,
    /// Layer multiplier, conventionally 1 / ln(m).
    pub ml: f32,
    /// What to measure.
    pub metric: Metric,

    /// Which strategy runs the math. Set from runtime.execution when the index is built.
    #[serde(skip)]
    pub mode: ExecutionMode,
}

impl Default for HnswConfig {
    fn default() -> Self {
        Self {
            m: 16,
            m_max: 32,
            ef_construction: 200,
            ef_search: 200,
            ml: 1.0 / 16.0_f32.ln(),
            metric: Metric::Cosine,
            mode: ExecutionMode::default(),
        }
    }
}

impl HnswConfig {
    /// Graph parameters derived from m, with the conventional m_max and ml.
    pub fn from_m(m: usize, ef_construction: usize, ef_search: usize) -> Self {
        Self {
            m,
            m_max: m * 2,
            ef_construction,
            ef_search,
            ml: 1.0 / (m as f32).ln(),
            ..Self::default()
        }
    }
}

/// Inverted-file index parameters.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IvfConfig {
    /// Partition count.
    pub num_clusters: usize,
    /// Partitions scanned per query. Higher is better recall and slower.
    pub num_probes: usize,
    /// Lloyd's-algorithm iteration cap when training centroids.
    pub max_iterations: usize,
    /// What to measure.
    pub metric: Metric,

    /// Which strategy runs the math. Set from runtime.execution when the index is built.
    #[serde(skip)]
    pub mode: ExecutionMode,
}

impl Default for IvfConfig {
    fn default() -> Self {
        Self {
            num_clusters: 100,
            num_probes: 5,
            max_iterations: 10,
            metric: Metric::Cosine,
            mode: ExecutionMode::default(),
        }
    }
}

impl IvfConfig {
    /// Cluster and probe counts sized for a collection of the given vector count.
    pub fn auto(num_vectors: usize) -> Self {
        Self::with_clusters((num_vectors as f32).sqrt().max(10.0) as usize)
    }

    /// The given cluster count, with the probe count sized from it.
    pub fn with_clusters(num_clusters: usize) -> Self {
        Self {
            num_clusters,
            num_probes: (num_clusters as f32 * 0.1).clamp(1.0, 10.0) as usize,
            ..Self::default()
        }
    }
}
