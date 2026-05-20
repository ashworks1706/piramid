// Supports: HNSW, Flat, IVF

pub mod flat;
pub mod hnsw;
pub mod ivf;
mod selector;
mod traits;

// Re-export trait and types
pub use selector::IndexConfig;
pub use traits::{IndexDetails, IndexStats, IndexType, SerializableIndex, VectorIndex};

// Re-export index implementations
pub use flat::{FlatConfig, FlatIndex};
pub use hnsw::{HnswConfig, HnswIndex, HnswStats};
pub use ivf::{IvfConfig, IvfIndex};
