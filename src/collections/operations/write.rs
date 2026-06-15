use uuid::Uuid;

use super::super::collection::Collection;
use super::limits;
use crate::error::Result;
use crate::metadata::Metadata;
use crate::storage::document::Document;
use crate::storage::record_store::RecordStore;
use crate::storage::wal::WalEntry;

pub fn insert_internal(storage: &mut Collection, entry: Document) -> Result<Uuid> {
    let id = entry.id;
    let raw_vec = entry.get_vector();
    let bytes = RecordStore::encode_document(&entry)?;

    limits::enforce_single(storage, bytes.len())?;
    let index_entry = storage.record_store.append(&bytes)?;
    storage.index.insert(id, index_entry.clone());

    storage.metadata.set_dimensions(raw_vec.len());

    if let Some(expected_dim) = storage.metadata.dimensions {
        crate::validation::validate_dimensions(&raw_vec, expected_dim)?;
    }

    storage.cache.put_vector(id, raw_vec.clone());
    storage.cache.put_metadata(id, entry.metadata.clone());
    storage.vector_index.insert(id, &raw_vec, &storage.cache);

    storage.metadata.update_vector_count(storage.index.len());

    Ok(id)
}

pub fn delete_internal(storage: &mut Collection, id: &Uuid) {
    storage.index.remove(id);
    storage.vector_index.remove(id);
    if storage.vector_index.index_type() != crate::index::IndexType::Hnsw {
        storage.cache.remove(id, true);
    } else {
        storage.cache.remove(id, false);
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
    storage.checkpoint.wal.log(&mut wal_entry)?;

    let id = insert_internal(storage, entry)?;
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
        storage.checkpoint.wal.log(&mut wal_entry)?;
    }

    let mut serialized: Vec<(Uuid, Vec<u8>)> = Vec::with_capacity(entries.len());
    let mut raw_vectors: Vec<(Uuid, Vec<f32>, Metadata)> = Vec::with_capacity(entries.len());
    for entry in &mut entries {
        let raw_vec = entry.get_vector();
        let metadata = entry.metadata.clone();
        let bytes = RecordStore::encode_document(entry)?;
        serialized.push((entry.id, bytes));
        raw_vectors.push((entry.id, raw_vec, metadata));
    }
    let total_bytes: u64 = serialized.iter().map(|(_, bytes)| bytes.len() as u64).sum();
    let max_entry_bytes = serialized.iter().map(|(_, bytes)| bytes.len()).max();
    limits::enforce_batch(storage, serialized.len(), total_bytes, max_entry_bytes)?;
    let pointers = storage.record_store.append_batch(&serialized)?;

    for ((id, _), pointer) in serialized.iter().zip(pointers) {
        storage.index.insert(*id, pointer);
        ids.push(*id);
    }

    storage.track_operation()?;

    for (id, vec_f32, metadata) in raw_vectors {
        storage.metadata.set_dimensions(vec_f32.len());
        if let Some(expected_dim) = storage.metadata.dimensions {
            crate::validation::validate_dimensions(&vec_f32, expected_dim)?;
        }
        storage.cache.put_metadata(id, metadata);
        storage.cache.put_vector(id, vec_f32.clone());
        storage.vector_index.insert(id, &vec_f32, &storage.cache);
    }
    storage.metadata.update_vector_count(storage.index.len());

    Ok(ids)
}

pub fn upsert(storage: &mut Collection, entry: Document) -> Result<Uuid> {
    let id = entry.id;
    let bytes = RecordStore::encode_document(&entry)?;

    let existing = storage.index.contains_key(&id);
    if existing {
        limits::enforce_single(storage, bytes.len())?;
        let vector = entry.get_vector();
        let mut wal_entry = WalEntry::Update {
            id,
            vector,
            text: entry.text.clone(),
            metadata: entry.metadata.clone(),
            seq: 0,
        };
        storage.checkpoint.wal.log(&mut wal_entry)?;

        delete_internal(storage, &id);
        insert_internal(storage, entry)?;
        storage.track_operation()?;
        Ok(id)
    } else {
        limits::enforce_single(storage, bytes.len())?;
        insert(storage, entry)
    }
}

pub fn delete(storage: &mut Collection, id: &Uuid) -> Result<bool> {
    if storage.index.contains_key(id) {
        let mut wal_entry = WalEntry::Delete { id: *id, seq: 0 };
        storage.checkpoint.wal.log(&mut wal_entry)?;

        delete_internal(storage, id);
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
            storage.checkpoint.wal.log(&mut wal_entry)?;
        }
    }

    for id in ids {
        if storage.index.contains_key(id) {
            delete_internal(storage, id);
            deleted_count += 1;
        }
    }

    if deleted_count > 0 {
        storage.track_operation()?;
    }

    Ok(deleted_count)
}
