//! Opening a collection: load sidecars, replay the WAL, rebuild what is missing.

/// Settings a collection is opened with.
#[derive(Clone, Default)]
pub struct CollectionOpenOptions {
    /// Configuration the collection runs with.
    pub config: piramid_core::config::CollectionConfig,
}

impl From<piramid_core::config::CollectionConfig> for CollectionOpenOptions {
    fn from(config: piramid_core::config::CollectionConfig) -> Self {
        Self { config }
    }
}

use std::collections::HashMap;
use uuid::Uuid;

use super::checkpoint::CheckpointManager;
use super::Collection;
use crate::cache::CacheManager;
use crate::index::load_vector_index;
use crate::index::HashMapVectorReader;
use crate::storage::manifest::CollectionMetadata;
use crate::storage::record_store::RecordStore;
use crate::storage::wal::{Wal, WalEntry};
use crate::storage::SidecarManager;
use piramid_core::error::{Result, StorageError};
use piramid_core::Document;

/// Open the collection at path, replaying the WAL and rebuilding sidecars as needed.
pub fn open(path: &str, options: CollectionOpenOptions) -> Result<Collection> {
    let config = options.config;

    // A path with no usable file stem is an error rather than a placeholder name.
    let collection_name = std::path::Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| {
            StorageError::InvalidPath(format!(
                "collection path '{path}' has no usable file stem to name the collection"
            ))
        })?
        .to_string();

    let sidecars = SidecarManager::at(path);
    let index = sidecars.load_offsets()?;
    let record_store = RecordStore::open(path, &config, &index)?;

    let manifest = match sidecars.load_manifest()? {
        Some(meta) => {
            let mut meta = meta;
            meta.update_vector_count(index.len());
            meta
        }
        None => CollectionMetadata::new(collection_name),
    };

    let loaded_vector_index = load_vector_index(path)?;
    let vector_index_missing = loaded_vector_index.is_none();
    let mut vector_index = loaded_vector_index.unwrap_or_else(|| {
        crate::index::create_index(&config.index, config.execution, index.len())
    });

    let min_seq = if config.wal.enabled {
        sidecars.load_wal_meta()?
    } else {
        0
    };
    let next_seq = min_seq + 1;

    let wal_path = sidecars.wal_path();

    let wal = if config.wal.enabled {
        Wal::new(wal_path.into(), next_seq, config.wal.sync_on_write)?
    } else {
        Wal::disabled(wal_path.into(), next_seq)?
    };

    let checkpoint = CheckpointManager::new(wal);

    let wal_entries = if config.wal.enabled {
        checkpoint.wal.replay(min_seq)?
    } else {
        Vec::new()
    };

    // Records exist but the ANN sidecar does not, so it is rebuilt from the record store.
    // Skipped when the WAL is about to replay, since replay reinserts every vector.
    if wal_entries.is_empty() && !index.is_empty() && vector_index_missing {
        rebuild_vector_index(&mut vector_index, &index, &record_store)?;
    }

    let mut collection = Collection {
        record_store,
        index,
        vector_index,
        cache: CacheManager::new(config.cache),
        config,
        manifest,
        path: path.to_string(),
        checkpoint,
    };

    // The WAL is cleared only once the checkpoint has made the replayed state durable.
    if !wal_entries.is_empty() {
        replay_wal(&mut collection, wal_entries)?;
        collection.rebuild_vector_cache()?;
        super::checkpoint::checkpoint(&mut collection)?;
        return Ok(collection);
    }

    collection.rebuild_vector_cache()?;
    Ok(collection)
}

fn replay_wal(collection: &mut Collection, entries: Vec<WalEntry>) -> Result<()> {
    for entry in entries {
        match entry {
            WalEntry::Insert {
                id,
                vector,
                text,
                metadata,
                ..
            } => {
                let document = Document {
                    id,
                    vector,
                    text,
                    metadata,
                };
                crate::document::insert_internal(collection, document)?;
            }
            // An update is a delete followed by an insert, so the ANN index sees the change.
            WalEntry::Update {
                id,
                vector,
                text,
                metadata,
                ..
            } => {
                crate::document::delete_internal(collection, &id);
                let document = Document {
                    id,
                    vector,
                    text,
                    metadata,
                };
                crate::document::insert_internal(collection, document)?;
            }
            WalEntry::Delete { id, .. } => {
                crate::document::delete_internal(collection, &id);
            }
            WalEntry::Checkpoint { .. } => {}
        }
    }
    Ok(())
}

fn rebuild_vector_index(
    vector_index: &mut Box<dyn crate::index::VectorIndex>,
    index: &HashMap<Uuid, crate::storage::sidecars::EntryPointer>,
    record_store: &RecordStore,
) -> Result<()> {
    // Read every live record through the offset index and re-insert it.
    let mut vectors: HashMap<Uuid, Vec<f32>> = HashMap::new();
    for (id, idx_entry) in index {
        let entry = record_store.read_document(idx_entry)?;
        vectors.insert(*id, entry.vector().to_vec());
    }

    let reader = HashMapVectorReader::new(&vectors);
    for (id, vector) in &vectors {
        vector_index.insert(*id, vector, &reader)?;
    }
    Ok(())
}
