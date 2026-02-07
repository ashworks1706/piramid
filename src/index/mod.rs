// Index module - unified interface for multiple indexing strategies
// Supports: HNSW, Flat, IVF

mod traits;
mod selector;
pub mod hnsw;
pub mod flat;
pub mod ivf;

// Re-export trait and types
pub use traits::{VectorIndex, IndexStats, IndexDetails, IndexType};
pub use selector::IndexConfig;

// Re-export index implementations
pub use hnsw::{HnswIndex, HnswConfig, HnswStats};
pub use flat::{FlatIndex, FlatConfig};
pub use ivf::{IvfIndex, IvfConfig};
