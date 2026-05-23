// Collection builder and initialization
use std::collections::HashMap;
use uuid::Uuid;

use super::cache::CacheManager;
use super::persistence::{load_wal_meta, PersistenceService};
use super::record_store::RecordStore;
use super::{storage::Collection, CollectionOpenOptions};
use crate::error::Result;
use crate::quantization::QuantizedVector;
use crate::storage::document::Document;
use crate::storage::metadata::CollectionMetadata;
use crate::storage::persistence::{get_wal_path, load_index, load_metadata, load_vector_index};
use crate::storage::wal::{Wal, WalEntry};

pub struct CollectionBuilder;

impl CollectionBuilder {
    pub fn open(path: &str, options: CollectionOpenOptions) -> Result<Collection> {
        let config = options.config;

        // Initialize Rayon thread pool based on config
        Collection::init_rayon_pool(&config.parallelism);

        // Derive collection name from file path
        let collection_name = std::path::Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Load existing index and metadata if they exist
        let index = load_index(path)?;
        let record_store = RecordStore::open(path, &config, &index)?;

        // If metadata exists, update vector count based on loaded index
        let metadata = match load_metadata(path)? {
            Some(meta) => {
                let mut meta = meta;
                meta.update_vector_count(index.len());
                meta
            }
            None => CollectionMetadata::new(collection_name),
        };

        // Load or create vector index
        let loaded_vector_index = load_vector_index(path)?;
        let vector_index_missing = loaded_vector_index.is_none();
        let mut vector_index = match loaded_vector_index {
            Some(loaded_index) => loaded_index,
            None => config.index.create_index(index.len()),
        };

        // If WAL is enabled, determine the minimum sequence number to replay from
        let min_seq = if config.wal.enabled {
            load_wal_meta(path)?
        } else {
            0
        };
        let next_seq = min_seq + 1;

        let wal_path = get_wal_path(path);

        // Initialize WAL and persistence service
        let wal = if config.wal.enabled {
            Wal::new(wal_path.into(), next_seq)?
        } else {
            Wal::disabled(wal_path.into(), next_seq)?
        };

        // Create persistence service which will handle WAL replay and checkpointing
        let persistence = PersistenceService::new(wal);

        // If WAL is enabled, replay entries from the WAL starting from the minimum sequence number
        let wal_entries = if config.wal.enabled {
            persistence.wal.replay(min_seq)?
        } else {
            Vec::new()
        };

        // If there are WAL entries to replay, we need to apply them to a temporary collection before checkpointing
        // why? Because we need to ensure that the collection state is consistent with the WAL entries before we can checkpoint and clear the WAL. By applying the WAL entries to a temporary collection, we can bring it up to date with all the changes recorded in the WAL, and then checkpoint that state to persist it. This way, we ensure that no changes are lost and that the collection is in sync with the WAL before we clear it.

        if !wal_entries.is_empty() {
            let mut temp_storage = Collection {
                record_store,
                index,
                vector_index,
                cache: CacheManager::new(config.cache),
                config: config.clone(),
                metadata,
                path: path.to_string(),
                persistence,
            };

            // Replay WAL entries to bring the collection up to date
            Self::replay_wal(&mut temp_storage, wal_entries)?;

            // After replaying, rebuild the vector cache to ensure it's in sync with the index
            temp_storage.rebuild_vector_cache();

            // Checkpoint the collection to persist the changes from the WAL replay, which will also clear the WAL
            super::persistence::checkpoint(&mut temp_storage)?;

            // After checkpointing, we can use the updated collection as our main collection instance
            return Ok(temp_storage);
        }

        // If the index is not empty but the vector index is missing, we need to rebuild the vector index from the existing data
        if !index.is_empty() && vector_index_missing {
            Self::rebuild_vector_index(&mut vector_index, &index, &record_store);
        }

        // Finally, create the collection instance with the loaded index, metadata, and vector index
        let mut collection = Collection {
            record_store,
            index,
            vector_index,
            cache: CacheManager::new(config.cache),
            config,
            metadata,
            path: path.to_string(),
            persistence,
        };

        collection.rebuild_vector_cache();
        Ok(collection)
    }

    fn replay_wal(storage: &mut Collection, entries: Vec<WalEntry>) -> Result<()> {
        // Apply each WAL entry to the collection. Inserts and updates will add or modify entries, while deletes will remove them.
        for entry in entries {
            match entry {
                // For inserts and updates, we create a Document from the WAL entry and insert it into the collection. Updates are treated as a delete followed by an insert to ensure the index is updated correctly.
                WalEntry::Insert {
                    id,
                    vector,
                    text,
                    metadata,
                    ..
                } => {
                    let vec_entry = Document {
                        id,
                        vector: QuantizedVector::from_f32_with_config(
                            &vector,
                            &storage.config.quantization,
                        ),
                        text,
                        metadata,
                    };
                    let _ = super::operations::insert_internal(storage, vec_entry);
                }

                WalEntry::Update {
                    id,
                    vector,
                    text,
                    metadata,
                    ..
                } => {
                    super::operations::delete_internal(storage, &id);
                    let vec_entry = Document {
                        id,
                        vector: QuantizedVector::from_f32_with_config(
                            &vector,
                            &storage.config.quantization,
                        ),
                        text,
                        metadata,
                    };
                    let _ = super::operations::insert_internal(storage, vec_entry);
                }
                WalEntry::Delete { id, .. } => {
                    super::operations::delete_internal(storage, &id);
                }
                WalEntry::Checkpoint { .. } => {}
            }
        }
        Ok(())
    }

    fn rebuild_vector_index(
        vector_index: &mut Box<dyn crate::index::VectorIndex>,
        index: &HashMap<Uuid, crate::storage::persistence::EntryPointer>,
        record_store: &RecordStore,
    ) {
        // If the vector index is missing but we have an existing index, we need to rebuild the vector index from the existing data. We read each entry from the memory-mapped file based on the offsets and lengths in the index, deserialize it into a Document, and then insert it into the vector index.
        let mut vectors: HashMap<Uuid, Vec<f32>> = HashMap::new();
        for (id, idx_entry) in index {
            if let Some(entry) = record_store.read_document(idx_entry) {
                vectors.insert(*id, entry.get_vector());
            }
        }

        // Once we have all the vectors loaded from the existing data, we can insert them into the vector index. This will rebuild the vector index so that it is in sync with the existing data in the collection.

        for (id, vector) in &vectors {
            vector_index.insert(*id, vector, &vectors);
        }
    }
}
