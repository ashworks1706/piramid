//! ANN index families: flat, HNSW, IVF, and the sidecar format they persist to.

mod contract;
pub mod flat;
pub mod hnsw;
pub mod ivf;
mod selector;
mod serialize;
mod sidecar;
mod stats;

pub use contract::{
    HashMapVectorReader, IndexSearchRequest, MetadataReader, VectorIndex, VectorReader, VectorSlab,
};
pub use piramid_core::config::{AutoIndexConfig, IndexConfig, IndexKind};
pub use selector::create_index;
pub(crate) use selector::{create_index_of_kind, growth_rank, kind_of};
pub use serialize::SerializableIndex;
pub use sidecar::{load_vector_index, save_vector_index};
pub use stats::{IndexDetails, IndexStats, IndexType};

pub use flat::FlatIndex;
pub use hnsw::{HnswIndex, HnswStats};
pub use ivf::IvfIndex;
pub use piramid_core::config::{FlatConfig, HnswConfig, IvfConfig};
