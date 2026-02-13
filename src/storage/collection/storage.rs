// Core Collection storage structure
use memmap2::MmapMut;
use std::collections::HashMap;
use std::fs::File;
use uuid::Uuid;

use crate::error::Result;
use crate::index::VectorIndex;
use crate::storage::persistence::EntryPointer;
use crate::storage::metadata::CollectionMetadata;
use super::persistence_service::PersistenceService;
use super::cache;

// Vector storage engine with memory-mapped files and pluggable indexing
pub struct Collection {
    pub(super) data_file: File,
    pub(super) mmap: Option<MmapMut>,
    pub(super) index: HashMap<Uuid, EntryPointer>,
    pub(super) vector_index: Box<dyn VectorIndex>,
    pub(super) vector_cache: HashMap<Uuid, Vec<f32>>,
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
        let mmap_size = self.mmap.as_ref().map(|m| m.len()).unwrap_or(0);
        let index_size = self.index.len() * std::mem::size_of::<(Uuid, EntryPointer)>();
        let vector_index_stats = self.vector_index.stats();
        
        mmap_size + index_size + vector_index_stats.memory_usage_bytes
    }

    pub fn vector_index(&self) -> &dyn VectorIndex {
        self.vector_index.as_ref()
    }

    pub fn vectors_view(&self) -> &HashMap<Uuid, Vec<f32>> {
        &self.vector_cache
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
