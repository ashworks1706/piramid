//! The on-disk form of an index.

use serde::{Deserialize, Serialize};

use super::contract::VectorIndex;

/// Persistable form of any index, one variant per family.
#[derive(Serialize, Deserialize)]
pub enum SerializableIndex {
    /// A flat index.
    Flat(crate::index::flat::FlatIndex),
    /// An HNSW index.
    Hnsw(crate::index::hnsw::HnswIndex),
    /// An IVF index.
    Ivf(crate::index::ivf::IvfIndex),
}

impl SerializableIndex {
    /// Restore a boxed trait object from the persisted form.
    pub fn to_trait_object(self) -> Box<dyn VectorIndex> {
        match self {
            SerializableIndex::Flat(idx) => Box::new(idx),
            SerializableIndex::Hnsw(idx) => Box::new(idx),
            SerializableIndex::Ivf(idx) => Box::new(idx),
        }
    }
}
