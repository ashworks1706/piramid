// Core Collection storage structure
use memmap2::MmapMut;
use std::collections::HashMap;
use std::fs::File;
use uuid::Uuid;

use crate::error::Result;
use crate::index::VectorIndex;
use crate::storage::persistence::EntryPointer;
use crate::storage::metadata::CollectionMetadata;
use super::persistence::PersistenceService;
use super::cache;

// Vector storage engine with memory-mapped files and pluggable indexing
pub struct Collection {
    pub(super) data_file: File,
    pub(super) mmap: Option<MmapMut>,
    pub(super) index: HashMap<Uuid, EntryPointer>,
    pub(super) vector_index: Box<dyn VectorIndex>,
    pub(super) vector_cache: HashMap<Uuid, Vec<f32>>,
    pub(super) metadata_cache: HashMap<Uuid, crate::metadata::Metadata>,
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
        let mmap_size = self.mmap.as_ref().map(|m| m.len()).unwrap_or(0); // Size of the memory-mapped file
        let index_size = self.index.capacity() * std::mem::size_of::<(Uuid, EntryPointer)>(); // Approximate size of the index based on its capacity

        let vector_cache_size = self.vector_cache.iter()
            .map(|(_, vec)| std::mem::size_of::<Uuid>() + vec.len() * std::mem::size_of::<f32>())
            .sum::<usize>(); // Size of the vector cache based on the number of entries and their lengths
        let metadata_cache_size = self.metadata_cache.len() * std::mem::size_of::<(Uuid, crate::metadata::Metadata)>(); // Approximate size of the metadata cache based on its capacity
        
        
        mmap_size + index_size + vector_cache_size + metadata_cache_size + self.vector_index.stats().memory_usage_bytes
    }

    pub fn vector_index(&self) -> &dyn VectorIndex {
        self.vector_index.as_ref()
    }

    pub fn vectors_view(&self) -> &HashMap<Uuid, Vec<f32>> {
        &self.vector_cache
    }

    pub fn metadata_view(&self) -> &HashMap<Uuid, crate::metadata::Metadata> {
        &self.metadata_cache
    }

    pub fn config(&self) -> &crate::config::CollectionConfig {
        &self.config
    }

    pub fn get_all(&self) -> Vec<crate::storage::document::Document> {
        let mut all_entries = Vec::new();
        for (id, _) in &self.index {
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
}
