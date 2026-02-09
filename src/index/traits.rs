// Index trait - unified interface for all indexing strategies
// All indexes (HNSW, Flat, IVF, etc.) implement this trait

use uuid::Uuid;
use std::collections::HashMap;
use crate::config::SearchConfig;
use serde::{Serialize, Deserialize};

// Core trait that all vector indexes must implement
// Provides a unified interface for insertion, search, and removal
pub trait VectorIndex: Send + Sync {
    // Insert a vector into the index
    // 
    // # Arguments
    // * `id` - Unique identifier for the vector
    // * `vector` - The vector to index
    // * `vectors` - All vectors in the collection (for distance calculations)
    fn insert(&mut self, id: Uuid, vector: &[f32], vectors: &HashMap<Uuid, Vec<f32>>);
    
    // Search for k nearest neighbors with default quality settings
    // 
    // # Arguments
    // * `query` - Query vector
    // * `k` - Number of neighbors to return
    // * `vectors` - All vectors in the collection
    // 
    // # Returns
    // Vector of IDs sorted by similarity (most similar first)
    // Search for k nearest neighbors with custom quality settings
    // 
    // # Arguments
    // * `query` - Query vector
    // * `k` - Number of neighbors to return
    // * `vectors` - All vectors in the collection
    // * `quality` - Search quality parameters (controls recall/speed tradeoff)
    // 
    // # Returns
    // Vector of IDs sorted by similarity (most similar first)
    fn search(&self, query: &[f32], k: usize, vectors: &HashMap<Uuid, Vec<f32>>, quality: SearchConfig) -> Vec<Uuid>;
    
    // Remove a vector from the index
    fn remove(&mut self, id: &Uuid);
    
    // Get index statistics
    fn stats(&self) -> IndexStats;
    
    // Get the index type name
    fn index_type(&self) -> IndexType;
}

// Statistics about an index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    pub index_type: IndexType,
    pub total_vectors: usize,
    pub memory_usage_bytes: usize,
    pub details: IndexDetails,
}

// Index-specific details
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum IndexDetails {
    Flat,
    Hnsw {
        max_layer: isize,
        layer_sizes: Vec<usize>,
        avg_connections: f32,
    },
    Ivf {
        num_clusters: usize,
        vectors_per_cluster: Vec<usize>,
        centroids_computed: bool,
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

impl SerializableIndex {
    pub fn to_trait_object(self) -> Box<dyn VectorIndex> {
        match self {
            SerializableIndex::Flat(idx) => Box::new(idx),
            SerializableIndex::Hnsw(idx) => Box::new(idx),
            SerializableIndex::Ivf(idx) => Box::new(idx),
        }
    }
}
