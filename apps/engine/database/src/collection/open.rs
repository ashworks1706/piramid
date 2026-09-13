//! Opening a collection: settle an interrupted compaction, load sidecars, replay the WAL.

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

use super::checkpoint::CheckpointManager;
use super::Collection;
use crate::resident::ResidentManager;
use crate::storage::manifest::CollectionMetadata;
use crate::storage::record_store::RecordStore;
use crate::storage::wal::{Wal, WalEntry};
use crate::storage::SidecarManager;
use piramid_core::error::{Result, StorageError};
use piramid_core::Document;

/// Opens the collection at path, finishing an interrupted compaction and replaying the WAL.
pub fn open(path: &str, options: CollectionOpenOptions) -> Result<Collection> {
    let config = options.config;

    // A path with no usable file stem is an error.
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
    let stored_manifest = sidecars.load_manifest()?;
    super::compact::recover(path)?;
    let offsets = sidecars.load_offsets()?;

    let created = stored_manifest.is_none();
    let manifest = match stored_manifest {
        Some(mut manifest) => {
            manifest.update_vector_count(offsets.len())?;
            manifest
        }
        None if offsets.is_empty() => {
            CollectionMetadata::new(collection_name.clone(), config.search.metric)?
        }
        None => {
            return Err(StorageError::CorruptedData(format!(
                "collection '{collection_name}' has {} documents but no manifest",
                offsets.len()
            ))
            .into())
        }
    };

    let record_store = RecordStore::open(path, &config, &offsets)?;
    if created {
        sidecars.save_manifest(&manifest)?;
    }

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
        Wal::disabled(wal_path.into(), next_seq)
    };

    let checkpoint = CheckpointManager::new(wal)?;

    let wal_entries = if config.wal.enabled {
        checkpoint.wal.replay(min_seq)?
    } else {
        Vec::new()
    };

    let mut collection = Collection {
        record_store,
        offsets,
        resident: ResidentManager::new(),
        config,
        manifest,
        path: path.to_string(),
        checkpoint,
        unfinished_compaction: None,
    };

    collection.load_resident()?;

    // The WAL is cleared only once the checkpoint has made the replayed state durable.
    if !wal_entries.is_empty() {
        replay_wal(&mut collection, wal_entries)?;
        super::checkpoint::checkpoint(&mut collection)?;
    }
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
            // An update replays as a delete followed by an insert.
            WalEntry::Update {
                id,
                vector,
                text,
                metadata,
                ..
            } => {
                crate::document::delete_internal(collection, &id)?;
                let document = Document {
                    id,
                    vector,
                    text,
                    metadata,
                };
                crate::document::insert_internal(collection, document)?;
            }
            WalEntry::Delete { id, .. } => {
                crate::document::delete_internal(collection, &id)?;
            }
            WalEntry::Checkpoint { .. } => {}
        }
    }
    Ok(())
}
