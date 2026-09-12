//! The collection object and the operations it exposes.

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

/// One open collection: its data file, offset index, caches, vector index, manifest and log.
pub struct Collection {
    pub(crate) record_store: RecordStore,
    pub(crate) index: HashMap<Uuid, EntryPointer>,
    pub(crate) vector_index: Box<dyn VectorIndex>,
    pub(crate) cache: CacheManager,
    /// Configuration the collection was opened with.
    pub config: piramid_core::config::CollectionConfig,
    /// Name, width, counts and timestamps.
    pub manifest: CollectionMetadata,
    /// Path of the data file; sidecar paths derive from it.
    pub path: String,
    /// Write-ahead log and checkpoint counters.
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

    /// Name, width, counts and timestamps.
    pub fn manifest(&self) -> &CollectionMetadata {
        &self.manifest
    }

    /// Number of live documents.
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

    /// The ANN index over the collection's vectors.
    pub fn vector_index(&self) -> &dyn VectorIndex {
        self.vector_index.as_ref()
    }

    /// Approximate bytes held by the vector store and metadata cache.
    pub fn cache_usage_bytes(&self) -> usize {
        self.cache.memory_usage_bytes()
    }

    /// Approximate bytes held by the metadata cache.
    pub fn metadata_cache_usage_bytes(&self) -> usize {
        self.cache.metadata_usage_bytes()
    }

    /// Empty the metadata cache and return the bytes freed.
    pub fn clear_metadata_cache(&mut self) -> usize {
        self.cache.clear_metadata()
    }

    /// Empty the vector store and metadata cache. Search cannot score until they are repopulated.
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

    /// The resident vectors, as indexes read them.
    pub fn vector_reader(&self) -> &dyn VectorReader {
        &self.cache
    }

    /// Metadata currently in the cache, keyed by id. Evicted documents are absent.
    pub fn metadata_view(&self) -> &HashMap<Uuid, piramid_core::metadata::Metadata> {
        self.cache.metadata()
    }

    /// Configuration the collection was opened with.
    pub fn config(&self) -> &piramid_core::config::CollectionConfig {
        &self.config
    }

    /// Up to limit documents in id order, skipping the first offset. Only the page is read.
    ///
    /// Errors when the offset index names a document the record store cannot return.
    pub fn page(&self, offset: usize, limit: usize) -> Result<Vec<piramid_core::Document>> {
        let mut ids: Vec<&Uuid> = self.index.keys().collect();
        ids.sort_unstable();
        ids.into_iter()
            .skip(offset)
            .take(limit)
            .map(|id| {
                crate::document::get(self, id)?.ok_or_else(|| {
                    piramid_core::error::StorageError::CorruptedIndex(format!(
                        "the offset index names document {id}, which the record store does not hold"
                    ))
                    .into()
                })
            })
            .collect()
    }

    /// Every document in id order. Errors like [Collection::page].
    pub fn get_all(&self) -> Result<Vec<piramid_core::Document>> {
        self.page(0, usize::MAX)
    }

    pub(crate) fn rebuild_vector_cache(&mut self) -> Result<()> {
        let mut cache = CacheManager::new(self.config.cache);
        for entry in self.get_all()? {
            cache.put_vector(entry.id, entry.vector())?;
            cache.put_metadata(entry.id, entry.metadata.clone());
        }
        self.cache = cache;
        Ok(())
    }

    /// Replace the index with the family an auto configuration picks for the current count, when
    /// the collection has grown past a threshold. A collection that shrinks keeps its family.
    pub(crate) fn grow_index_family(&mut self) -> Result<()> {
        use crate::index::IndexType;
        use piramid_core::config::IndexKind;

        let rank_of_kind = |kind: IndexKind| match kind {
            IndexKind::Flat => 0,
            IndexKind::Ivf => 1,
            IndexKind::Hnsw => 2,
        };
        let rank_of_type = |kind: IndexType| match kind {
            IndexType::Flat => 0,
            IndexType::Ivf => 1,
            IndexType::Hnsw => 2,
        };
        let count = self.index.len();
        let wanted = self.config.index.select_type(count);
        let current = self.vector_index.index_type();
        if rank_of_kind(wanted) <= rank_of_type(current) {
            return Ok(());
        }

        tracing::info!(
            target: "piramid::indexing",
            collection = self.path.as_str(),
            vectors = count,
            from = %current,
            to = ?wanted,
            "index_family_grown"
        );
        let mut grown =
            crate::index::create_index(&self.config.index, self.config.execution, count);
        let ids: Vec<Uuid> = self.index.keys().copied().collect();
        for id in ids {
            let vector = VectorReader::get(&self.cache, &id).ok_or_else(|| {
                piramid_core::error::IndexError::BuildFailed(format!(
                    "vector {id} is not resident, so the index cannot grow into a new family"
                ))
            })?;
            grown.insert(id, vector, &self.cache)?;
        }
        self.vector_index = grown;
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
    /// Open or create the collection at path with default configuration.
    pub fn open(path: &str) -> Result<Self> {
        super::open::open(path, CollectionOpenOptions::default())
    }

    /// Open or create the collection at path with the given options.
    pub fn open_with_options(path: &str, options: CollectionOpenOptions) -> Result<Self> {
        super::open::open(path, options)
    }

    /// The document with id, or None when absent.
    pub fn get(&self, id: &Uuid) -> Result<Option<Document>> {
        crate::document::get(self, id)
    }

    /// Log and store a new document and return its id.
    pub fn insert(&mut self, entry: Document) -> Result<Uuid> {
        crate::document::insert(self, entry)
    }

    /// Log and store new documents and return their ids.
    pub fn insert_batch(&mut self, entries: Vec<Document>) -> Result<Vec<Uuid>> {
        crate::document::insert_batch(self, entries)
    }

    /// Replace the document with the same id, or insert it when absent, and return its id.
    pub fn upsert(&mut self, entry: Document) -> Result<Uuid> {
        crate::document::upsert(self, entry)
    }

    /// Remove the document with id. False when it was absent.
    pub fn delete(&mut self, id: &Uuid) -> Result<bool> {
        crate::document::delete(self, id)
    }

    /// Remove the documents with the given ids and return how many were present.
    pub fn delete_batch(&mut self, ids: &[Uuid]) -> Result<usize> {
        crate::document::delete_batch(self, ids)
    }

    /// Replace the metadata of the document with id. False when it was absent.
    pub fn update_metadata(&mut self, id: &Uuid, metadata: Metadata) -> Result<bool> {
        crate::document::update_metadata(self, id, metadata)
    }

    /// Replace the vector of the document with id. False when it was absent.
    pub fn update_vector(&mut self, id: &Uuid, vector: Vec<f32>) -> Result<bool> {
        crate::document::update_vector(self, id, vector)
    }

    /// The k best hits for query, scored under metric.
    pub fn search(
        &self,
        query: &[f32],
        k: usize,
        metric: Metric,
        params: crate::search::SearchParams,
    ) -> Result<Vec<Hit>> {
        super::search_target::search(self, query, k, metric, params)
    }

    /// The k best hits for each query, scored under metric, one list per query.
    pub fn search_batch_with(
        &self,
        queries: &[Vec<f32>],
        k: usize,
        metric: Metric,
        params: crate::search::SearchParams,
    ) -> Result<Vec<Vec<Hit>>> {
        super::search_target::search_batch(self, queries, k, metric, params)
    }

    /// Save the sidecars, then mark and rotate the write-ahead log.
    pub fn checkpoint(&mut self) -> Result<()> {
        super::checkpoint::checkpoint(self)
    }

    /// Drain buffered log entries to the kernel.
    pub fn flush(&mut self) -> Result<()> {
        super::checkpoint::flush(self)
    }
}
