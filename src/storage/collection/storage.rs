use std::collections::HashMap;
use uuid::Uuid;

use super::cache::{self, CacheManager};
use super::persistence::PersistenceService;
use super::record_store::RecordStore;
use crate::error::Result;
use crate::index::VectorIndex;
use crate::storage::metadata::CollectionMetadata;
use crate::storage::persistence::{
    get_wal_path, save_vector_index, warm_file, EntryPointer,
};

pub struct Collection {
    pub(super) record_store: RecordStore,
    pub(super) index: HashMap<Uuid, EntryPointer>,
    pub(super) vector_index: Box<dyn VectorIndex>,
    pub(super) cache: CacheManager,
    pub config: crate::config::CollectionConfig,
    pub metadata: CollectionMetadata,
    pub path: String,
    pub persistence: PersistenceService,
}

impl Collection {
    pub(super) fn init_rayon_pool(config: &crate::config::ParallelismConfig) {
        let num_threads = config.num_threads();
        if num_threads > 0 {
            rayon::ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .build_global()
                .ok();
        }
    }

    // Track operations to trigger checkpoints based on WAL config
    pub(super) fn track_operation(&mut self) -> Result<()> {
        let interval_due = if let Some(last) = self.persistence.last_checkpoint() {
            if let Some(interval) = self.config.wal.checkpoint_interval_secs {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                now.saturating_sub(last) >= interval
            } else {
                false
            }
        } else {
            false
        };

        if self.persistence.should_checkpoint(&self.config.wal) || interval_due {
            super::persistence::checkpoint(self)?;
            self.persistence.reset_counter();
        }
        Ok(())
    }

    pub fn metadata(&self) -> &CollectionMetadata {
        &self.metadata
    }

    pub fn count(&self) -> usize {
        self.index.len()
    }

    pub fn memory_usage_bytes(&self) -> usize {
        // Calculate memory usage by summing the sizes of the memory-mapped file, index, vector cache, metadata cache, and vector index.
        let mmap_size = self.record_store.mapped_len();
        let index_size = self.index.capacity() * std::mem::size_of::<(Uuid, EntryPointer)>(); // Approximate size of the index based on its capacity

        mmap_size
            + index_size
            + self.cache.memory_usage_bytes()
            + self.vector_index.stats().memory_usage_bytes
    }

    pub fn vector_index(&self) -> &dyn VectorIndex {
        self.vector_index.as_ref()
    }

    pub fn cache_usage_bytes(&self) -> usize {
        self.cache.memory_usage_bytes()
    }

    pub fn metadata_cache_usage_bytes(&self) -> usize {
        self.cache.metadata_usage_bytes()
    }

    pub fn clear_metadata_cache(&mut self) -> usize {
        self.cache.clear_metadata()
    }

    pub fn clear_caches_for_rebuild(&mut self) {
        self.cache.clear_all();
    }

    /// Fault frequently used files into the page cache to reduce cold-start latency.
    pub fn warm_page_cache(&self) {
        self.record_store.warm_page_cache();
        let base = self.path.clone();
        let _ = warm_file(&format!("{}.vecindex.db", base));
        let _ = warm_file(&format!("{}.index.db", base));
        let _ = warm_file(&get_wal_path(&base));
    }

    pub fn vectors_view(&self) -> &HashMap<Uuid, Vec<f32>> {
        self.cache.vectors()
    }

    pub fn metadata_view(&self) -> &HashMap<Uuid, crate::metadata::Metadata> {
        self.cache.metadata()
    }

    pub fn config(&self) -> &crate::config::CollectionConfig {
        &self.config
    }

    pub fn get_all(&self) -> Vec<crate::storage::document::Document> {
        let mut all_entries = Vec::new();
        for id in self.index.keys() {
            if let Some(entry) = super::operations::get(self, id) {
                all_entries.push(entry);
            }
        }
        all_entries
    }

    pub(super) fn rebuild_vector_cache(&mut self) {
        cache::rebuild(self);
    }

    // If cache and index diverge (e.g., after crash), rebuild to ensure consistency.
    pub fn ensure_cache_consistency(&mut self) {
        cache::ensure_consistent(self);
    }

    /// Rebuild the vector index from on-disk data and persist it.
    pub fn rebuild_index(&mut self) -> Result<()> {
        // Collect all vectors from storage
        let mut vectors: HashMap<Uuid, Vec<f32>> = HashMap::new();

        for (id, pointer) in &self.index {
            if let Some(entry) = self.record_store.read_document(pointer) {
                vectors.insert(*id, entry.get_vector());
            }
        }

        // Build fresh index
        let mut new_index = self.config.index.create_index(self.index.len());
        for (id, vec) in &vectors {
            new_index.insert(*id, vec, &vectors);
        }

        // Swap and persist
        self.vector_index = new_index;
        self.rebuild_vector_cache();
        save_vector_index(self.path.as_str(), self.vector_index())?;
        Ok(())
    }
}
