// Collection builder and initialization
use std::collections::HashMap;
use std::fs::OpenOptions;
use uuid::Uuid;

use crate::error::Result;
use crate::storage::wal::{Wal, WalEntry};
use crate::storage::persistence::{
    get_wal_path, ensure_file_size, create_mmap, load_index,
    load_metadata, load_vector_index
};
use crate::storage::document::Document;
use crate::storage::metadata::CollectionMetadata;
use crate::quantization::QuantizedVector;
use super::storage::Collection;
use super::persistence::PersistenceService;

pub struct CollectionBuilder;

impl CollectionBuilder {
    pub fn open(path: &str) -> Result<Collection> {
        Self::with_config(path, crate::config::CollectionConfig::default())
    }

    pub fn with_config(path: &str, config: crate::config::CollectionConfig) -> Result<Collection> {
        Collection::init_rayon_pool(&config.parallelism);
        
        let collection_name = std::path::Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        let initial_size = if config.memory.use_mmap {
            config.memory.initial_mmap_size as u64
        } else {
            1024 * 1024
        };
        
        ensure_file_size(&file, initial_size)?;
        let mmap = if config.memory.use_mmap {
            Some(create_mmap(&file)?)
        } else {
            None
        };

        let index = load_index(path)?;

        let metadata = match load_metadata(path)? {
            Some(meta) => {
                let mut meta = meta;
                meta.update_vector_count(index.len());
                meta
            }
            None => CollectionMetadata::new(collection_name),
        };

        let mut vector_index = match load_vector_index(path)? {
            Some(loaded_index) => loaded_index,
            None => config.index.create_index(index.len())
        };
        
        let wal_path = get_wal_path(path);
        let wal = if config.wal.enabled {
            Wal::new(wal_path.into())?
        } else {
            Wal::disabled(wal_path.into())?
        };
        let persistence = PersistenceService::new(wal);
        
        let wal_entries = if config.wal.enabled {
            persistence.wal.replay()?
        } else {
            Vec::new()
        };
        
        if !wal_entries.is_empty() {
        let mut temp_storage = Collection {
            data_file: file,
            mmap,
            index,
            vector_index,
            vector_cache: HashMap::new(),
            config: config.clone(),
            metadata,
            path: path.to_string(),
            persistence,
        };
            
        Self::replay_wal(&mut temp_storage, wal_entries)?;
        temp_storage.rebuild_vector_cache();
            
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            temp_storage.persistence.wal.checkpoint(timestamp)?;
            super::persistence::save_index(&temp_storage)?;
            super::persistence::save_vector_index(&temp_storage)?;
            temp_storage.persistence.wal.truncate()?;
            
            return Ok(temp_storage);
        }
        
        if !index.is_empty() && load_vector_index(path)?.is_none() {
            if let Some(ref mmap_ref) = mmap {
                Self::rebuild_vector_index(&mut vector_index, &index, mmap_ref);
            }
        }
        
        let mut collection = Collection {
            data_file: file,
            mmap,
            index,
            vector_index,
            vector_cache: HashMap::new(),
            config,
            metadata,
            path: path.to_string(),
            persistence,
        };
        collection.rebuild_vector_cache();
        Ok(collection)
    }

    fn replay_wal(storage: &mut Collection, entries: Vec<WalEntry>) -> Result<()> {
        for entry in entries {
            match entry {
                WalEntry::Insert { id, vector, text, metadata } => {
                    let vec_entry = Document {
                        id,
                        vector: QuantizedVector::from_f32(&vector),
                        text,
                        metadata,
                    };
                    let _ = super::operations::insert_internal(storage, vec_entry);
                }
                WalEntry::Update { id, vector, text, metadata } => {
                    super::operations::delete_internal(storage, &id);
                    let vec_entry = Document {
                        id,
                        vector: QuantizedVector::from_f32(&vector),
                        text,
                        metadata,
                    };
                    let _ = super::operations::insert_internal(storage, vec_entry);
                }
                WalEntry::Delete { id } => {
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
        mmap_ref: &memmap2::MmapMut
    ) {
        let mut vectors: HashMap<Uuid, Vec<f32>> = HashMap::new();
        for (id, idx_entry) in index {
            let offset = idx_entry.offset as usize;
            let length = idx_entry.length as usize;
            if offset + length <= mmap_ref.len() {
                let bytes = &mmap_ref[offset..offset + length];
                if let Ok(entry) = bincode::deserialize::<Document>(bytes) {
                    vectors.insert(*id, entry.get_vector());
                }
            }
        }
        
        for (id, vector) in &vectors {
            vector_index.insert(*id, vector, &vectors);
        }
    }
}
