//! The caching domain entry.

use crate::storage::vectors::VectorReader;
use crate::{MetadataCache, VectorStore};
use piramid_core::config::CacheConfig;
use piramid_core::metadata::Metadata;
use uuid::Uuid;

/// The caching domain entry for one collection.
///
/// Owns the [VectorStore] and the [MetadataCache].
pub struct CacheManager {
    store: VectorStore,
    metadata: MetadataCache,
}

impl CacheManager {
    /// Empty store and cache, bounded by config.
    pub fn new(config: CacheConfig) -> Self {
        Self {
            store: VectorStore::new(),
            metadata: MetadataCache::new(config),
        }
    }

    /// The resident vector store, as a [VectorReader] for indexes.
    pub fn vector_reader(&self) -> &dyn VectorReader {
        &self.store
    }

    /// All cached metadata, keyed by id.
    pub fn metadata(&self) -> &std::collections::HashMap<Uuid, Metadata> {
        self.metadata.entries()
    }

    /// Insert or replace the resident vector for id.
    pub fn put_vector(&mut self, id: Uuid, vector: &[f32]) -> piramid_core::error::Result<()> {
        self.store.put(id, vector)
    }

    /// Cache metadata for id, evicting oldest entries past the configured bound.
    pub fn put_metadata(&mut self, id: Uuid, metadata: Metadata) {
        self.metadata.put(id, metadata);
    }

    /// Drop id from the metadata cache, and from the store when remove_vector is set.
    ///
    /// remove_vector is false for an HNSW delete, which tombstones the node and keeps its vector
    /// resident.
    pub fn remove(&mut self, id: &Uuid, remove_vector: bool) {
        if remove_vector {
            self.store.remove(id);
        }
        self.metadata.remove(id);
    }

    /// Clear the store and the cache both. Only correct before a rebuild repopulates the store.
    pub fn clear_all(&mut self) {
        self.store.clear();
        self.metadata.clear();
    }

    /// Clear the metadata cache only, returning the bytes freed. The store is untouched.
    pub fn clear_metadata(&mut self) -> usize {
        self.metadata.clear()
    }

    /// Approximate bytes held by the store and the cache together.
    pub fn memory_usage_bytes(&self) -> usize {
        self.store.usage_bytes() + self.metadata.usage_bytes()
    }

    /// Approximate bytes held by the metadata cache alone.
    pub fn metadata_usage_bytes(&self) -> usize {
        self.metadata.usage_bytes()
    }
}

/// Forwards every method of the trait to the store.
impl VectorReader for CacheManager {
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
