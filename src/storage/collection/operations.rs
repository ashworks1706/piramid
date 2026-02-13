// Collection CRUD operations
use uuid::Uuid;

use crate::error::Result;
use crate::storage::document::Document;
use crate::storage::wal::WalEntry;
use crate::storage::persistence::{EntryPointer, grow_mmap_if_needed};
use crate::quantization::QuantizedVector;
use crate::metadata::Metadata;
use super::storage::Collection;

pub fn get(storage: &Collection, id: &Uuid) -> Option<Document> {
    let index_entry = storage.index.get(id)?;
    let offset = index_entry.offset as usize;
    let length = index_entry.length as usize;
    let bytes = &storage.mmap.as_ref().unwrap()[offset..offset + length];
    bincode::deserialize(bytes).ok()
}

pub fn insert_internal(storage: &mut Collection, entry: Document) -> Result<Uuid> {
    let id = entry.id;
    let bytes = bincode::serialize(&entry)?;

    let offset = storage.index.values()
        .map(|idx| idx.offset + idx.length as u64)
        .max()
        .unwrap_or(0);

    let required_size = offset + bytes.len() as u64;
    grow_mmap_if_needed(&mut storage.mmap, &storage.data_file, required_size)?;
    
    let mmap = storage.mmap.as_mut().unwrap();
    mmap[offset as usize..(offset as usize + bytes.len())]
        .copy_from_slice(&bytes);
    
    let index_entry = EntryPointer::new(offset, bytes.len() as u32);
    storage.index.insert(id, index_entry);
    
    let vec_f32 = entry.get_vector();
    
    storage.metadata.set_dimensions(vec_f32.len());
    
    if let Some(expected_dim) = storage.metadata.dimensions {
        crate::validation::validate_dimensions(&vec_f32, expected_dim)?;
    }
    
    storage.vector_cache.insert(id, vec_f32.clone());
    storage.vector_index.insert(id, &vec_f32, &storage.vector_cache);
    
    storage.metadata.update_vector_count(storage.index.len());
    
    Ok(id)
}

pub fn delete_internal(storage: &mut Collection, id: &Uuid) {
    storage.index.remove(id);
    storage.vector_index.remove(id);
    storage.vector_cache.remove(id);
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
    
    super::persistence::save_index(storage)?;
    storage.track_operation()?;

    insert_internal(storage, entry)
}

pub fn upsert(storage: &mut Collection, entry: Document) -> Result<Uuid> {
    let id = entry.id;
    if storage.index.contains_key(&id) {
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
        insert_internal(storage, entry)?;
        super::persistence::save_index(storage)?;
        super::persistence::save_vector_index(storage)?;
        storage.track_operation()?;
        Ok(id)
    } else {
        insert(storage, entry)
    }
}

pub fn insert_batch(storage: &mut Collection, entries: Vec<Document>) -> Result<Vec<Uuid>> {
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
        storage.persistence.wal.log(&mut wal_entry)?;
    }
    
    let mut serialized: Vec<(Uuid, Vec<u8>)> = Vec::with_capacity(entries.len());
    for entry in &entries {
        let bytes = bincode::serialize(entry)?;
        serialized.push((entry.id, bytes));
    }
    
    let current_offset = storage.index.values()
        .map(|idx| idx.offset + idx.length as u64)
        .max()
        .unwrap_or(0);
    
    let total_bytes: u64 = serialized.iter().map(|(_, b)| b.len() as u64).sum();
    let required_size = current_offset + total_bytes;
    
    grow_mmap_if_needed(&mut storage.mmap, &storage.data_file, required_size)?;
    
    let mut offset = current_offset;
    let mmap = storage.mmap.as_mut().unwrap();
    
    for (id, bytes) in &serialized {
        mmap[offset as usize..(offset as usize + bytes.len())]
            .copy_from_slice(bytes);
        
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
    
    for entry in entries {
        let vec_f32 = entry.get_vector();
        storage.vector_cache.insert(entry.id, vec_f32.clone());
        storage.vector_index.insert(entry.id, &vec_f32, &storage.vector_cache);
    }
    
    Ok(ids)
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
        let vector = entry.get_vector();
        
        let mut wal_entry = WalEntry::Update {
            id: *id,
            vector,
            text: entry.text.clone(),
            metadata: metadata.clone(),
            seq: 0,
        };
        storage.persistence.wal.log(&mut wal_entry)?;
        
        let mut entry = entry;
        entry.metadata = metadata;
        delete(storage, id)?;
        insert(storage, entry)?;
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
        storage.persistence.wal.log(&mut wal_entry)?;
        
        let mut entry = entry;
        entry.vector = QuantizedVector::from_f32(&vector);
        delete(storage, id)?;
        
        insert(storage, entry)?;
        Ok(true)
    } else {
        Ok(false)
    }
}
