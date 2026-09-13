//! The collection object and the operations it exposes.

use std::collections::HashMap;
use uuid::Uuid;

use crate::CollectionOpenOptions;
use piramid_core::metadata::Metadata;
use piramid_core::{Document, Hit};
use piramid_hardware::compute::Metric;

use super::checkpoint::CheckpointManager;
use crate::resident::ResidentManager;
use crate::storage::manifest::CollectionMetadata;
use crate::storage::record_store::RecordStore;
use crate::storage::sidecars::{warm_file, EntryPointer};
use crate::storage::vectors::VectorReader;
use crate::storage::SidecarManager;
use piramid_core::error::{ConfigError, Result, StorageError};

/// One open collection: its data file, offsets, resident vectors and metadata, manifest and log.
pub struct Collection {
    pub(crate) record_store: RecordStore,
    pub(crate) offsets: HashMap<Uuid, EntryPointer>,
    pub(crate) resident: ResidentManager,
    /// Config the collection runs with; search.metric is not the metric it scores with.
    pub config: piramid_core::config::CollectionConfig,
    /// Name, metric, width, counts and timestamps.
    pub manifest: CollectionMetadata,
    /// Path of the data file; sidecar paths derive from it.
    pub path: String,
    /// Write-ahead log and checkpoint counters.
    pub checkpoint: CheckpointManager,
    /// Why a committed compaction could not finish in memory; while set, writes are refused.
    pub(crate) unfinished_compaction: Option<String>,
}

impl Collection {
    pub(crate) fn track_operation(&mut self) -> Result<()> {
        let now = piramid_core::clock::unix_secs()?;
        if self.checkpoint.should_checkpoint(&self.config.wal, now)? {
            super::checkpoint::checkpoint(self)?;
            self.checkpoint.reset_counter();
        }
        Ok(())
    }

    /// First setting in next that needs a reopen, named by its path; None if next applies live.
    pub fn setting_needing_reopen(
        &self,
        next: &piramid_core::config::CollectionConfig,
    ) -> Option<&'static str> {
        let current = &self.config;
        [
            (
                current.quantization != next.quantization,
                "runtime.quantization",
            ),
            (current.memory != next.memory, "runtime.memory"),
            (current.hardware != next.hardware, "startup.hardware"),
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

    /// Applies live settings (parallel, limits, WAL, execution); errors if next needs a reopen.
    pub fn apply_live_settings(
        &mut self,
        next: &piramid_core::config::CollectionConfig,
    ) -> Result<()> {
        if let Some(setting) = self.setting_needing_reopen(next) {
            return Err(ConfigError::Invalid(format!(
                "{setting} changed; it applies when the collection is opened"
            ))
            .into());
        }
        self.config.search.parallel = next.search.parallel;
        self.config.limits = next.limits;
        self.config.wal = next.wal;
        self.config.execution = next.execution;
        Ok(())
    }

    /// Name, metric, width, counts and timestamps.
    pub fn manifest(&self) -> &CollectionMetadata {
        &self.manifest
    }

    /// The metric the collection was created with, which every search of it scores with.
    pub fn metric(&self) -> Metric {
        self.manifest.metric
    }

    /// Number of live documents.
    pub fn count(&self) -> usize {
        self.offsets.len()
    }

    /// Approximate resident size: mmap plus offsets plus resident vectors and metadata.
    pub fn memory_usage_bytes(&self) -> Result<usize> {
        let offsets_size = self.offsets.capacity() * std::mem::size_of::<(Uuid, EntryPointer)>();

        Ok(self.record_store.mapped_len()? + offsets_size + self.resident.memory_usage_bytes())
    }

    /// Faults the data file, the offsets and the WAL into the page cache.
    pub fn warm_page_cache(&self) {
        self.record_store.warm_page_cache();
        let sidecars = SidecarManager::at(&self.path);
        for path in [sidecars.offsets_path(), sidecars.wal_path()] {
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

    /// The resident vectors of every live document.
    pub fn vector_reader(&self) -> &dyn VectorReader {
        &self.resident
    }

    /// The resident metadata of every live document, keyed by id.
    pub fn metadata_view(&self) -> &HashMap<Uuid, Metadata> {
        self.resident.metadata()
    }

    /// Configuration the collection runs with.
    pub fn config(&self) -> &piramid_core::config::CollectionConfig {
        &self.config
    }

    /// Up to limit documents in id order, skipping offset; errors on an offset with no document.
    pub fn page(&self, offset: usize, limit: usize) -> Result<Vec<Document>> {
        let mut ids: Vec<&Uuid> = self.offsets.keys().collect();
        ids.sort_unstable();
        ids.into_iter()
            .skip(offset)
            .take(limit)
            .map(|id| {
                crate::document::get(self, id)?.ok_or_else(|| {
                    piramid_core::error::StorageError::CorruptedSidecar(format!(
                        "the offsets name document {id}, which the record store does not hold"
                    ))
                    .into()
                })
            })
            .collect()
    }

    /// Every document in id order. Errors like [Collection::page].
    pub fn get_all(&self) -> Result<Vec<Document>> {
        self.page(0, usize::MAX)
    }

    /// Replace the resident vectors and metadata with those of every document the offsets name.
    pub(crate) fn load_resident(&mut self) -> Result<()> {
        let mut resident = ResidentManager::new();
        for (id, pointer) in &self.offsets {
            let document = self.record_store.read_document(pointer)?;
            resident.put_vector(*id, document.vector())?;
            resident.put_metadata(*id, document.metadata);
        }
        self.resident = resident;
        Ok(())
    }

    /// Refuse a write while a committed compaction is unfinished in memory.
    pub(crate) fn ensure_writable(&self) -> Result<()> {
        match &self.unfinished_compaction {
            None => Ok(()),
            Some(reason) => Err(StorageError::CompactionUnfinished {
                collection: self.manifest.name.clone(),
                reason: reason.clone(),
            }
            .into()),
        }
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

    /// The k best hits for query matching the filter in params; errors if metric is wrong for it.
    pub fn search(
        &self,
        query: &[f32],
        k: usize,
        metric: Metric,
        params: crate::search::SearchParams,
    ) -> Result<Vec<Hit>> {
        super::search_target::search(self, query, k, metric, params)
    }

    /// The k best hits for each query, one list per query. Errors like [Collection::search].
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
