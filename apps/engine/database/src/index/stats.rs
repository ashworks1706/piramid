//! What an index reports about its own shape.

use serde::{Deserialize, Serialize};

/// Statistics about an index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    /// Index family.
    pub index_type: IndexType,
    /// Number of indexed vectors.
    pub total_vectors: usize,
    /// Approximate resident size in bytes.
    pub memory_usage_bytes: usize,
    /// Family-specific detail.
    pub details: IndexDetails,
}

/// Family-specific index statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum IndexDetails {
    /// Flat indexes have no structure to report.
    Flat,
    /// Graph shape for HNSW.
    Hnsw {
        /// Highest occupied layer.
        max_layer: isize,
        /// Node count per layer.
        layer_sizes: Vec<usize>,
        /// Mean out-degree.
        avg_connections: f32,
        /// Candidate list width a search uses when the query sets none.
        ef_search: usize,
    },
    /// Partition shape for IVF.
    Ivf {
        /// Number of partitions.
        num_clusters: usize,
        /// Vectors assigned to each partition.
        vectors_per_cluster: Vec<usize>,
        /// Whether centroids have been trained.
        centroids_computed: bool,
        /// Partitions a search scans when the query sets none.
        num_probes: usize,
    },
}

/// Supported index families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexType {
    /// Brute-force linear scan, O(N). Best under 10k vectors.
    Flat,
    /// Hierarchical Navigable Small World graph, O(log N). Best above 100k vectors.
    Hnsw,
    /// Inverted file index, O(sqrt N). Best between 10k and 1M vectors.
    Ivf,
}

impl std::fmt::Display for IndexType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexType::Flat => write!(f, "Flat"),
            IndexType::Hnsw => write!(f, "HNSW"),
            IndexType::Ivf => write!(f, "IVF"),
        }
    }
}
