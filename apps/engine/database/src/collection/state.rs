use std::collections::HashMap;
use uuid::Uuid;

use crate::CollectionOpenOptions;
use piramid_core::metadata::Metadata;
use piramid_core::{Document, Hit};
use piramid_hardware::compute::Metric;

use super::checkpoint::CheckpointManager;
use crate::cache::CacheManager;
use crate::index::save_vector_index;
use crate::index::{HashMapVectorReader, VectorIndex, VectorReader};
use crate::storage::manifest::CollectionMetadata;
use crate::storage::record_store::RecordStore;
use crate::storage::sidecars::{warm_file, EntryPointer};
use crate::storage::SidecarManager;
use piramid_core::error::Result;

pub struct Collection {
    pub(crate) record_store: RecordStore,
    pub(crate) index: HashMap<Uuid, EntryPointer>,
    pub(crate) vector_index: Box<dyn VectorIndex>,
    pub(crate) cache: CacheManager,
    pub config: piramid_core::config::CollectionConfig,
    pub manifest: CollectionMetadata,
    pub path: String,
    pub checkpoint: CheckpointManager,
}

impl Collection {
    pub(crate) fn track_operation(&mut self) -> Result<()> {
        let now = piramid_core::clock::unix_secs();
        if self.checkpoint.should_checkpoint(&self.config.wal, now)? {
            super::checkpoint::checkpoint(self)?;
            self.checkpoint.reset_counter();
        }
        Ok(())
    }

    /// The first setting in next that differs from this collection and takes effect only when
    /// the collection is opened, named by its config path. None when next can be applied live.
    pub fn setting_needing_reopen(
        &self,
        next: &piramid_core::config::CollectionConfig,
    ) -> Option<&'static str> {
        let current = &self.config;
        let mut metadata_cache = next.cache.metadata;
        metadata_cache.max_bytes = current.cache.metadata.max_bytes;
        [
            (current.index != next.index, "runtime.index"),
            (
                current.quantization != next.quantization,
                "runtime.quantization",
            ),
            (current.memory != next.memory, "runtime.memory"),
            (current.hardware != next.hardware, "startup.hardware"),
            (
                current.cache.vectors != next.cache.vectors,
                "runtime.cache.vectors",
            ),
            (
                current.cache.metadata != metadata_cache,
                "runtime.cache.metadata",
            ),
            (
                current.wal.enabled != next.wal.enabled,
                "runtime.wal.enabled",
            ),
            (
                current.wal.sync_on_write != next.wal.sync_on_write,
                "runtime.wal.sync_on_write",
            ),
        ]
        .into_iter()
        .find_map(|(differs, name)| differs.then_some(name))
    }

    /// Apply the settings an open collection reads as it runs: search, limits, WAL checkpoint
    /// thresholds, the metadata cache budget and the execution mode.
    ///
    /// Errors, changing nothing, when next differs in a setting that needs a reopen.
    pub fn apply_live_settings(
        &mut self,
        next: &piramid_core::config::CollectionConfig,
    ) -> Result<()> {
        if let Some(setting) = self.setting_needing_reopen(next) {
            return Err(piramid_core::error::IndexError::InvalidConfig(format!(
                "{setting} changed; it applies when the collection is opened"
            ))
            .into());
        }
        self.config.search = next.search;
        self.config.limits = next.limits;
        self.config.wal = next.wal;
        self.config.cache.metadata.max_bytes = next.cache.metadata.max_bytes;
        self.config.execution = next.execution;
        self.vector_index.set_execution(next.execution);
        Ok(())
    }

    pub fn manifest(&self) -> &CollectionMetadata {
        &self.manifest
    }

    pub fn count(&self) -> usize {
        self.index.len()
    }

    /// Approximate resident size: mmap plus offset index plus caches plus ANN structure.
    pub fn memory_usage_bytes(&self) -> Result<usize> {
        let index_size = self.index.capacity() * std::mem::size_of::<(Uuid, EntryPointer)>();

        Ok(self.record_store.mapped_len()?
            + index_size
            + self.cache.memory_usage_bytes()
            + self.vector_index.stats().memory_usage_bytes)
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

    /// Faults frequently used files into the page cache to reduce cold-start latency.
    pub fn warm_page_cache(&self) {
        self.record_store.warm_page_cache();
        let sidecars = SidecarManager::at(&self.path);
        for path in [
            sidecars.vector_index_path(),
            sidecars.offsets_path(),
            sidecars.wal_path(),
        ] {
            if let Err(error) = warm_file(&path) {
                tracing::warn!(
                    target: "piramid::collections",
                    %path,
                    %error,
                    "could not warm page cache for file"
                );
            }
        }
    }

    pub fn vector_reader(&self) -> &dyn VectorReader {
        &self.cache
    }

    pub fn metadata_view(&self) -> &HashMap<Uuid, piramid_core::metadata::Metadata> {
        self.cache.metadata()
    }

    pub fn config(&self) -> &piramid_core::config::CollectionConfig {
        &self.config
    }

    pub fn get_all(&self) -> Result<Vec<piramid_core::Document>> {
        let mut all_entries = Vec::new();
        for id in self.index.keys() {
            if let Some(entry) = crate::document::get(self, id)? {
                all_entries.push(entry);
            }
        }
        Ok(all_entries)
    }

    pub(crate) fn rebuild_vector_cache(&mut self) -> Result<()> {
        let mut cache = CacheManager::new(self.config.cache);
        for id in self.index.keys() {
            if let Some(entry) = crate::document::get(self, id)? {
                cache.put_vector(*id, entry.vector())?;
                cache.put_metadata(*id, entry.metadata.clone());
            }
        }
        self.cache = cache;
        Ok(())
    }

    /// Rebuild the vector index from on-disk data and persist it.
    pub fn rebuild_index(&mut self) -> Result<()> {
        let mut vectors: HashMap<Uuid, Vec<f32>> = HashMap::new();

        for (id, pointer) in &self.index {
            let entry = self.record_store.read_document(pointer)?;
            vectors.insert(*id, entry.vector().to_vec());
        }

        let mut new_index =
            crate::index::create_index(&self.config.index, self.config.execution, self.index.len());
        let reader = HashMapVectorReader::new(&vectors);
        for (id, vec) in &vectors {
            new_index.insert(*id, vec, &reader)?;
        }

        self.vector_index = new_index;
        self.rebuild_vector_cache()?;
        save_vector_index(self.path.as_str(), self.vector_index())?;
        Ok(())
    }
}

impl Collection {
    pub fn open(path: &str) -> Result<Self> {
        super::open::open(path, CollectionOpenOptions::default())
    }

    pub fn open_with_options(path: &str, options: CollectionOpenOptions) -> Result<Self> {
        super::open::open(path, options)
    }

    pub fn get(&self, id: &Uuid) -> Result<Option<Document>> {
        crate::document::get(self, id)
    }

    pub fn insert(&mut self, entry: Document) -> Result<Uuid> {
        crate::document::insert(self, entry)
    }

    pub fn insert_batch(&mut self, entries: Vec<Document>) -> Result<Vec<Uuid>> {
        crate::document::insert_batch(self, entries)
    }

    pub fn upsert(&mut self, entry: Document) -> Result<Uuid> {
        crate::document::upsert(self, entry)
    }

    pub fn delete(&mut self, id: &Uuid) -> Result<bool> {
        crate::document::delete(self, id)
    }

    pub fn delete_batch(&mut self, ids: &[Uuid]) -> Result<usize> {
        crate::document::delete_batch(self, ids)
    }

    pub fn update_metadata(&mut self, id: &Uuid, metadata: Metadata) -> Result<bool> {
        crate::document::update_metadata(self, id, metadata)
    }

    pub fn update_vector(&mut self, id: &Uuid, vector: Vec<f32>) -> Result<bool> {
        crate::document::update_vector(self, id, vector)
    }

    pub fn search(
        &self,
        query: &[f32],
        k: usize,
        metric: Metric,
        params: crate::search::SearchParams,
    ) -> Result<Vec<Hit>> {
        super::search_target::search(self, query, k, metric, params)
    }

    pub fn search_batch_with(
        &self,
        queries: &[Vec<f32>],
        k: usize,
        metric: Metric,
        params: crate::search::SearchParams,
    ) -> Result<Vec<Vec<Hit>>> {
        super::search_target::search_batch(self, queries, k, metric, params)
    }

    pub fn checkpoint(&mut self) -> Result<()> {
        super::checkpoint::checkpoint(self)
    }

    pub fn flush(&mut self) -> Result<()> {
        super::checkpoint::flush(self)
    }
}
