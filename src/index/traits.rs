// All indexes (HNSW, Flat, IVF, etc.) implement this trait

use crate::config::SearchConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

pub trait VectorReader {
    fn get(&self, id: &Uuid) -> Option<&[f32]>;
    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = (Uuid, &'a [f32])> + 'a>;
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub struct HashMapVectorReader<'a> {
    vectors: &'a HashMap<Uuid, Vec<f32>>,
}

impl<'a> HashMapVectorReader<'a> {
    pub fn new(vectors: &'a HashMap<Uuid, Vec<f32>>) -> Self {
        Self { vectors }
    }
}

impl VectorReader for HashMapVectorReader<'_> {
    fn get(&self, id: &Uuid) -> Option<&[f32]> {
        self.vectors.get(id).map(Vec::as_slice)
    }

    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = (Uuid, &'a [f32])> + 'a> {
        Box::new(
            self.vectors
                .iter()
                .map(|(id, vector)| (*id, vector.as_slice())),
        )
    }

    fn len(&self) -> usize {
        self.vectors.len()
    }
}

pub trait VectorIndex: Send + Sync {
    // Insert a vector into the index

    // # Arguments
    // * id - Unique identifier for the vector
    // * vector - The vector to index
    // * vectors - All vectors in the collection (for distance calculations)
    fn insert(&mut self, id: Uuid, vector: &[f32], vectors: &dyn VectorReader);

    // Search for k nearest neighbors with custom quality settings
    // # Arguments
    // * query- Query vector
    // * k - Number of neighbors to return
    // * vectors - All vectors in the collection
    // * quality - Search quality parameters (controls recall/speed tradeoff)
    //
    // # Returns
    // Vector of IDs sorted by similarity (most similar first)
    fn search(
        &self,
        query: &[f32],
        k: usize,
        vectors: &dyn VectorReader,
        quality: SearchConfig,
        filter: Option<&crate::search::query::Filter>,
        metadatas: &HashMap<Uuid, crate::metadata::Metadata>,
    ) -> Vec<Uuid>;

    // Remove a vector from the index
    fn remove(&mut self, id: &Uuid);

    // Get index statistics
    fn stats(&self) -> IndexStats;

    // Get the index type name
    fn index_type(&self) -> IndexType;

    // Convert the index into a serializable form for persistence
    fn to_serializable(&self) -> SerializableIndex;
}

// Statistics about an index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    pub index_type: IndexType,     // Type of index (Flat, HNSW, IVF)
    pub total_vectors: usize,      // Total number of vectors indexed
    pub memory_usage_bytes: usize, // Approximate memory usage of the index in bytes
    pub details: IndexDetails, // Index-specific details (e.g. HNSW layer sizes, IVF cluster counts)
}

// Index-specific details
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum IndexDetails {
    Flat,
    Hnsw {
        max_layer: isize,        // Maximum layer in the HNSW graph
        layer_sizes: Vec<usize>, // Number of nodes in each layer
        avg_connections: f32,    // Average number of connections per node
    },
    Ivf {
        num_clusters: usize,             // Number of clusters in the IVF index
        vectors_per_cluster: Vec<usize>, // Number of vectors assigned to each cluster
        centroids_computed: bool,        // Whether centroids have been computed for the clusters
    },
}

// Supported index types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexType {
    // Brute force linear scan - O(N), best for <10k vectors
    Flat,
    // Hierarchical Navigable Small World - O(log N), best for >100k vectors
    Hnsw,
    // Inverted File Index - O(√N), best for 10k-1M vectors
    Ivf,
}

// better readability in logs and stats
impl std::fmt::Display for IndexType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexType::Flat => write!(f, "Flat"),
            IndexType::Hnsw => write!(f, "HNSW"),
            IndexType::Ivf => write!(f, "IVF"),
        }
    }
}

// Wrapper for persisting any index type
#[derive(Serialize, Deserialize)]
pub enum SerializableIndex {
    Flat(crate::index::flat::FlatIndex),
    Hnsw(crate::index::hnsw::HnswIndex),
    Ivf(crate::index::ivf::IvfIndex),
}
// Implement a method to convert the SerializableIndex back into a trait object for use to persist the index state and later restore it while still using the unified VectorIndex interface for operations.
impl SerializableIndex {
    pub fn to_trait_object(self) -> Box<dyn VectorIndex> {
        match self {
            SerializableIndex::Flat(idx) => Box::new(idx),
            SerializableIndex::Hnsw(idx) => Box::new(idx),
            SerializableIndex::Ivf(idx) => Box::new(idx),
        }
    }
}
