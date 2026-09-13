//! The resident state domain entry.

use std::collections::HashMap;

use super::{MetadataStore, VectorStore};
use crate::storage::vectors::VectorReader;
use piramid_core::metadata::Metadata;
use uuid::Uuid;

/// The resident state of one collection: a [VectorStore] and [MetadataStore] with matching entries.
#[derive(Default)]
pub struct ResidentManager {
    store: VectorStore,
    metadata: MetadataStore,
}

impl ResidentManager {
    /// Empty vector and metadata stores.
    pub fn new() -> Self {
        Self::default()
    }

    /// The resident vector store, as a [VectorReader].
    pub fn vector_reader(&self) -> &dyn VectorReader {
        &self.store
    }

    /// The metadata of every live document, keyed by id.
    pub fn metadata(&self) -> &HashMap<Uuid, Metadata> {
        self.metadata.entries()
    }

    /// Insert or replace the resident vector for id.
    pub fn put_vector(&mut self, id: Uuid, vector: &[f32]) -> piramid_core::error::Result<()> {
        self.store.put(id, vector)
    }

    /// Insert or replace the resident metadata for id.
    pub fn put_metadata(&mut self, id: Uuid, metadata: Metadata) {
        self.metadata.put(id, metadata);
    }

    /// Drop the vector and the metadata of id.
    pub fn remove(&mut self, id: &Uuid) {
        self.store.remove(id);
        self.metadata.remove(id);
    }

    /// Approximate bytes held by the vector store and the metadata store together.
    pub fn memory_usage_bytes(&self) -> usize {
        self.store.usage_bytes() + self.metadata.usage_bytes()
    }
}

/// Forwards every method of the trait to the store.
impl VectorReader for ResidentManager {
    fn get(&self, id: &Uuid) -> Option<&[f32]> {
        self.store.get(id)
    }

    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = (Uuid, &'a [f32])> + 'a> {
        self.store.iter()
    }

    fn len(&self) -> usize {
        VectorReader::len(&self.store)
    }

    fn dim(&self) -> Option<usize> {
        VectorReader::dim(&self.store)
    }

    fn as_slab(&self) -> Option<crate::storage::vectors::VectorSlab<'_>> {
        self.store.as_slab()
    }

    fn gather_into(&self, ids: &[Uuid], out: &mut [f32]) -> Option<()> {
        self.store.gather_into(ids, out)
    }
}
