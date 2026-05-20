// Collection CRUD operations.
use uuid::Uuid;

use crate::error::{Result, ServerError};
use crate::storage::document::Document;
use crate::storage::wal::WalEntry;
use crate::storage::persistence::{EntryPointer, grow_mmap_if_needed};
use crate::quantization::QuantizedVector;
use crate::metadata::Metadata;
use super::storage::Collection;

fn write_entry_bytes(storage: &mut Collection, offset: u64, bytes: &[u8]) -> Result<()> {
    if let Some(mmap) = storage.mmap.as_mut() {
        let start = offset as usize;
        let end = start + bytes.len();
        mmap[start..end].copy_from_slice(bytes);
    } else {
        use std::io::{Seek, SeekFrom, Write};
        storage.data_file.seek(SeekFrom::Start(offset))?;
        storage.data_file.write_all(bytes)?;
    }
    Ok(())
}

fn append_entry(storage: &mut Collection, bytes: &[u8]) -> Result<EntryPointer> {
    let offset = storage
        .index
        .values()
        .map(|idx| idx.offset + idx.length as u64)
        .max()
        .unwrap_or(0);

    let required_size = offset + bytes.len() as u64;
    grow_mmap_if_needed(&mut storage.mmap, &storage.data_file, required_size)?;
    write_entry_bytes(storage, offset, bytes)?;

    Ok(EntryPointer::new(offset, bytes.len() as u32))
}

fn persist_after_update(storage: &mut Collection, save_vector_index: bool) -> Result<()> {
    super::persistence::save_index(storage)?;
    if save_vector_index {
        super::persistence::save_vector_index(storage)?;
    }
    storage.track_operation()?;
    Ok(())
}

fn enforce_limits_single(storage: &Collection, entry_bytes: usize) -> Result<()> {
    let limits = storage.config.limits;

    if let Some(max_vecs) = limits.max_vectors {
        if storage.count() >= max_vecs {
            return Err(ServerError::InvalidRequest("Collection max vectors reached".into()).into());
        }
    }

    if let Some(max_bytes) = limits.max_bytes {
        let current_size = storage.data_file.metadata()?.len();
        let required = current_size.saturating_add(entry_bytes as u64);
        if required > max_bytes {
            return Err(ServerError::InvalidRequest("Collection max size reached".into()).into());
        }
    }

    if let Some(max_vec_bytes) = limits.max_vector_bytes {
        if entry_bytes > max_vec_bytes {
            return Err(ServerError::InvalidRequest("Vector exceeds max allowed size".into()).into());
        }
    }

    Ok(())
}

// current + batch <= max 
fn enforce_limits_batch(storage: &Collection, total_entries: usize, total_bytes: u64, max_entry_bytes: Option<usize>) -> Result<()> {
    let limits = storage.config.limits;

    if let Some(max_vecs) = limits.max_vectors {
        let current = storage.count();
        if current.saturating_add(total_entries) > max_vecs {
            return Err(ServerError::InvalidRequest("Collection max vectors reached".into()).into());
        }
    }
    
    if let Some(max_bytes) = limits.max_bytes {
        let current_size = storage.data_file.metadata()?.len();
        let required = current_size.saturating_add(total_bytes);
        if required > max_bytes {
            return Err(ServerError::InvalidRequest("Collection max size reached".into()).into());
        }
    }

    if let Some(max_vec_bytes) = max_entry_bytes {
        if max_vec_bytes > 0 {
            if let Some(cfg_limit) = limits.max_vector_bytes {
                if max_vec_bytes > cfg_limit {
                    return Err(ServerError::InvalidRequest("Vector exceeds max allowed size".into()).into());
                }
            }
        }
    }

    Ok(())
}

pub fn get(storage: &Collection, id: &Uuid) -> Option<Document> {
    let index_entry = storage.index.get(id)?;
    let offset = index_entry.offset as usize;
    let length = index_entry.length as usize;
    if let Some(mmap) = storage.mmap.as_ref() {
        let bytes = &mmap[offset..offset + length];
        bincode::deserialize(bytes).ok()
    } else {
        use std::io::{Read, Seek, SeekFrom};
        let mut file = storage.data_file.try_clone().ok()?;
        let mut buf = vec![0u8; length];
        file.seek(SeekFrom::Start(index_entry.offset)).ok()?;
        file.read_exact(&mut buf).ok()?;
        bincode::deserialize(&buf).ok()
    }
}

pub fn insert_internal(storage: &mut Collection, mut entry: Document) -> Result<Uuid> {
    let id = entry.id;
    let raw_vec = entry.get_vector();
    entry.vector = QuantizedVector::from_f32_with_config(&raw_vec, &storage.config.quantization);
    let bytes = bincode::serialize(&entry)?; 

    enforce_limits_single(storage, bytes.len())?;
    let index_entry = append_entry(storage, &bytes)?;
    storage.index.insert(id, index_entry.clone());

    storage.metadata.set_dimensions(raw_vec.len());

    // Keep the collection dimension aligned with the first inserted vector.
    if let Some(expected_dim) = storage.metadata.dimensions {
        crate::validation::validate_dimensions(&raw_vec, expected_dim)?;
    }

    storage.vector_cache.insert(id, raw_vec.clone());
    storage.vector_index.insert(id, &raw_vec, &storage.vector_cache);
    
    storage.metadata.update_vector_count(storage.index.len());
    
    Ok(id)
}

pub fn delete_internal(storage: &mut Collection, id: &Uuid) {
    storage.index.remove(id);
    storage.vector_index.remove(id);
    if storage.vector_index.index_type() != crate::index::IndexType::Hnsw {
        storage.vector_cache.remove(id);
        storage.metadata_cache.remove(id);
    }
    storage.metadata.update_vector_count(storage.index.len());
}

pub fn insert(storage: &mut Collection, entry: Document) -> Result<Uuid> {
    let vector = entry.get_vector();
    let mut wal_entry = WalEntry::Insert { 
        id: entry.id, 
        vector,
        text: entry.text.clone(),
        metadata: entry.metadata.clone(),
        seq: 0,
    };
    storage.persistence.wal.log(&mut wal_entry)?;

    let id = insert_internal(storage, entry)?;
    super::persistence::save_index(storage)?;
    storage.track_operation()?;
    Ok(id)
}

pub fn insert_batch(storage: &mut Collection, mut entries: Vec<Document>) -> Result<Vec<Uuid>> {
    let mut ids = Vec::with_capacity(entries.len());

    for entry in &entries {
        let vector = entry.get_vector();
        let mut wal_entry = WalEntry::Insert {
            id: entry.id,
            vector,
            text: entry.text.clone(),
            metadata: entry.metadata.clone(),
            seq: 0,
        };
        // Log each insert before applying the batch.
        storage.persistence.wal.log(&mut wal_entry)?;
    }

    let mut serialized: Vec<(Uuid, Vec<u8>)> = Vec::with_capacity(entries.len());
    let mut raw_vectors: Vec<(Uuid, Vec<f32>)> = Vec::with_capacity(entries.len());
    for entry in &mut entries {
        let raw_vec = entry.get_vector();
        entry.vector = QuantizedVector::from_f32_with_config(&raw_vec, &storage.config.quantization);
        let bytes = bincode::serialize(entry)?;
        serialized.push((entry.id, bytes));
        raw_vectors.push((entry.id, raw_vec));
    }
    let current_offset = storage.index.values()
        .map(|idx| idx.offset + idx.length as u64)
        .max()
        .unwrap_or(0);

    let total_bytes: u64 = serialized.iter().map(|(_, b)| b.len() as u64).sum();
    let max_entry_bytes = serialized.iter().map(|(_, b)| b.len()).max();
    enforce_limits_batch(storage, serialized.len(), total_bytes, max_entry_bytes)?;
    let required_size = current_offset + total_bytes;

    grow_mmap_if_needed(&mut storage.mmap, &storage.data_file, required_size)?;

    let mut offset = current_offset;

    for (id, bytes) in &serialized {
        write_entry_bytes(storage, offset, bytes)?;

        let index_entry = EntryPointer {
            offset,
            length: bytes.len() as u32,
        };
        storage.index.insert(*id, index_entry);
        ids.push(*id);
        
        offset += bytes.len() as u64;
    }

    super::persistence::save_index(storage)?;
    storage.track_operation()?;

    for (id, vec_f32) in raw_vectors {
        storage.metadata.set_dimensions(vec_f32.len());
        if let Some(expected_dim) = storage.metadata.dimensions {
            crate::validation::validate_dimensions(&vec_f32, expected_dim)?;
        }
        storage.vector_cache.insert(id, vec_f32.clone());
        storage.vector_index.insert(id, &vec_f32, &storage.vector_cache);
    }
    storage.metadata.update_vector_count(storage.index.len());
    
    Ok(ids)
}

pub fn upsert(storage: &mut Collection, mut entry: Document) -> Result<Uuid> {
    let id = entry.id;
    let raw_vec = entry.get_vector();
    entry.vector = QuantizedVector::from_f32_with_config(&raw_vec, &storage.config.quantization);
    let bytes = bincode::serialize(&entry)?;

    let existing = storage.index.contains_key(&id);
    if existing {
        enforce_limits_single(storage, bytes.len())?;
        let vector = entry.get_vector();
        let mut wal_entry = WalEntry::Update {
            id,
            vector,
            text: entry.text.clone(),
            metadata: entry.metadata.clone(),
            seq: 0,
        };
        storage.persistence.wal.log(&mut wal_entry)?;

        delete_internal(storage, &id);
        // Rehydrate the document from the serialized bytes for the insert path.
        let doc = bincode::deserialize(&bytes)?;
        insert_internal(storage, doc)?;
        super::persistence::save_index(storage)?;
        super::persistence::save_vector_index(storage)?;
        storage.track_operation()?;
        Ok(id)
    } else {
        enforce_limits_single(storage, bytes.len())?;
        insert(storage, entry)
    }
}

pub fn delete(storage: &mut Collection, id: &Uuid) -> Result<bool> {
    if storage.index.contains_key(id) {
        let mut wal_entry = WalEntry::Delete { id: *id, seq: 0 };
        storage.persistence.wal.log(&mut wal_entry)?;

        delete_internal(storage, id);
        super::persistence::save_index(storage)?;
        super::persistence::save_vector_index(storage)?;
        storage.track_operation()?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn delete_batch(storage: &mut Collection, ids: &[Uuid]) -> Result<usize> {
    let mut deleted_count = 0;

    for id in ids {
        if storage.index.contains_key(id) {
            let mut wal_entry = WalEntry::Delete { id: *id, seq: 0 };
            // Log each delete before applying the batch.
            storage.persistence.wal.log(&mut wal_entry)?;
        }
    }

    for id in ids {
        if storage.index.contains_key(id) {
            delete_internal(storage, id);
            deleted_count += 1;
        }
    }
    
    if deleted_count > 0 {
        super::persistence::save_index(storage)?;
        super::persistence::save_vector_index(storage)?;
        storage.track_operation()?;
    }
    
    Ok(deleted_count)
}

pub fn update_metadata(storage: &mut Collection, id: &Uuid, metadata: Metadata) -> Result<bool> {
    if let Some(entry) = get(storage, id) {
        let mut wal_entry = WalEntry::Update {
            id: *id,
            vector: entry.get_vector(),
            text: entry.text.clone(),
            metadata: metadata.clone(),
            seq: 0,
        };
        // Persist the metadata update in the WAL first.
        storage.persistence.wal.log(&mut wal_entry)?;

        let mut entry = entry;
        entry.metadata = metadata.clone();
        let bytes = bincode::serialize(&entry)?;

        enforce_limits_single(storage, bytes.len())?;
        let index_entry = append_entry(storage, &bytes)?;
        storage.index.insert(*id, index_entry);
        storage.metadata_cache.insert(*id, metadata);
        storage.metadata.update_vector_count(storage.index.len());
        persist_after_update(storage, false)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn update_vector(storage: &mut Collection, id: &Uuid, vector: Vec<f32>) -> Result<bool> {
    if let Some(entry) = get(storage, id) {
        let mut wal_entry = WalEntry::Update {
            id: *id,
            vector: vector.clone(),
            text: entry.text.clone(),
            metadata: entry.metadata.clone(),
            seq: 0,
        };
        // Persist the vector update in the WAL first.
        storage.persistence.wal.log(&mut wal_entry)?;

        let mut entry = entry;
        entry.vector = QuantizedVector::from_f32_with_config(&vector, &storage.config.quantization);

        if let Some(expected_dim) = storage.metadata.dimensions {
            crate::validation::validate_dimensions(&vector, expected_dim)?;
        } else {
            storage.metadata.set_dimensions(vector.len());
        }

        let bytes = bincode::serialize(&entry)?;
        enforce_limits_single(storage, bytes.len())?;

        let index_entry = append_entry(storage, &bytes)?;
        storage.index.insert(*id, index_entry);
        storage.vector_cache.insert(*id, vector.clone());
        storage.vector_index.remove(id);
        storage.vector_index.insert(*id, &vector, &storage.vector_cache);
        storage.metadata.update_vector_count(storage.index.len());
        persist_after_update(storage, true)?;
        Ok(true)
    } else {
        Ok(false)
    }
}
