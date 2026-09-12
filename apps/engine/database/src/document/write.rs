use uuid::Uuid;

use super::super::collection::Collection;
use super::read::get;
use crate::collection::limits;
use crate::storage::record_store::RecordStore;
use crate::storage::wal::WalEntry;
use piramid_core::error::Result;
use piramid_core::metadata::Metadata;
use piramid_core::Document;

pub fn insert_internal(collection: &mut Collection, entry: Document) -> Result<Uuid> {
    let id = entry.id;
    let raw_vec = entry.vector().to_vec();
    let bytes = RecordStore::encode_document(&entry)?;

    limits::enforce_single(collection, bytes.len())?;
    let index_entry = collection.record_store.append(&bytes)?;
    collection.index.insert(id, index_entry.clone());

    collection.manifest.set_dimensions(raw_vec.len())?;

    if let Some(expected_dim) = collection.manifest.dimensions {
        piramid_core::validation::validate_dimensions(&raw_vec, expected_dim)?;
    }

    collection.cache.put_vector(id, &raw_vec)?;
    collection.cache.put_metadata(id, entry.metadata.clone());
    collection
        .vector_index
        .insert(id, &raw_vec, &collection.cache)?;

    collection
        .manifest
        .update_vector_count(collection.index.len());
    collection.grow_index_family()?;

    Ok(id)
}

pub fn delete_internal(collection: &mut Collection, id: &Uuid) {
    collection.index.remove(id);
    collection.vector_index.remove(id);
    if collection.vector_index.index_type() != crate::index::IndexType::Hnsw {
        collection.cache.remove(id, true);
    } else {
        collection.cache.remove(id, false);
    }
    collection
        .manifest
        .update_vector_count(collection.index.len());
}

fn insert_wal_entry(entry: &Document) -> WalEntry {
    WalEntry::Insert {
        id: entry.id,
        vector: entry.vector().to_vec(),
        text: entry.text.clone(),
        metadata: entry.metadata.clone(),
        seq: 0,
    }
}

pub fn insert(collection: &mut Collection, entry: Document) -> Result<Uuid> {
    let mut wal_entry = insert_wal_entry(&entry);
    collection.checkpoint.wal.log(&mut wal_entry)?;

    let id = insert_internal(collection, entry)?;
    collection.track_operation()?;
    Ok(id)
}

pub fn insert_batch(collection: &mut Collection, mut entries: Vec<Document>) -> Result<Vec<Uuid>> {
    let mut ids = Vec::with_capacity(entries.len());

    for entry in &entries {
        let mut wal_entry = insert_wal_entry(entry);
        collection.checkpoint.wal.log(&mut wal_entry)?;
    }

    let mut serialized: Vec<(Uuid, Vec<u8>)> = Vec::with_capacity(entries.len());
    let mut raw_vectors: Vec<(Uuid, Vec<f32>, Metadata)> = Vec::with_capacity(entries.len());
    for entry in &mut entries {
        let raw_vec = entry.vector().to_vec();
        let metadata = entry.metadata.clone();
        let bytes = RecordStore::encode_document(entry)?;
        serialized.push((entry.id, bytes));
        raw_vectors.push((entry.id, raw_vec, metadata));
    }
    let total_bytes: u64 = serialized.iter().map(|(_, bytes)| bytes.len() as u64).sum();
    let max_entry_bytes = serialized.iter().map(|(_, bytes)| bytes.len()).max();
    limits::enforce_batch(collection, serialized.len(), total_bytes, max_entry_bytes)?;
    let pointers = collection.record_store.append_batch(&serialized)?;

    for ((id, _), pointer) in serialized.iter().zip(pointers) {
        collection.index.insert(*id, pointer);
        ids.push(*id);
    }

    collection.track_operation()?;

    for (id, vec_f32, metadata) in raw_vectors {
        collection.manifest.set_dimensions(vec_f32.len())?;
        if let Some(expected_dim) = collection.manifest.dimensions {
            piramid_core::validation::validate_dimensions(&vec_f32, expected_dim)?;
        }
        collection.cache.put_metadata(id, metadata);
        collection.cache.put_vector(id, &vec_f32)?;
        collection
            .vector_index
            .insert(id, &vec_f32, &collection.cache)?;
    }
    collection
        .manifest
        .update_vector_count(collection.index.len());

    Ok(ids)
}

pub fn upsert(collection: &mut Collection, entry: Document) -> Result<Uuid> {
    let id = entry.id;
    let bytes = RecordStore::encode_document(&entry)?;
    limits::enforce_single(collection, bytes.len())?;

    if !collection.index.contains_key(&id) {
        return insert(collection, entry);
    }

    let mut wal_entry = WalEntry::Update {
        id,
        vector: entry.vector().to_vec(),
        text: entry.text.clone(),
        metadata: entry.metadata.clone(),
        seq: 0,
    };
    collection.checkpoint.wal.log(&mut wal_entry)?;

    delete_internal(collection, &id);
    insert_internal(collection, entry)?;
    collection.track_operation()?;
    Ok(id)
}

pub fn delete(collection: &mut Collection, id: &Uuid) -> Result<bool> {
    if collection.index.contains_key(id) {
        let mut wal_entry = WalEntry::Delete { id: *id, seq: 0 };
        collection.checkpoint.wal.log(&mut wal_entry)?;

        delete_internal(collection, id);
        collection.track_operation()?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn delete_batch(collection: &mut Collection, ids: &[Uuid]) -> Result<usize> {
    let mut deleted_count = 0;

    for id in ids {
        if collection.index.contains_key(id) {
            let mut wal_entry = WalEntry::Delete { id: *id, seq: 0 };
            collection.checkpoint.wal.log(&mut wal_entry)?;
        }
    }

    for id in ids {
        if collection.index.contains_key(id) {
            delete_internal(collection, id);
            deleted_count += 1;
        }
    }

    if deleted_count > 0 {
        collection.track_operation()?;
    }

    Ok(deleted_count)
}

pub fn update_vector(collection: &mut Collection, id: &Uuid, vector: Vec<f32>) -> Result<bool> {
    if let Some(entry) = get(collection, id)? {
        let mut wal_entry = WalEntry::Update {
            id: *id,
            vector: vector.clone(),
            text: entry.text.clone(),
            metadata: entry.metadata.clone(),
            seq: 0,
        };
        collection.checkpoint.wal.log(&mut wal_entry)?;

        let mut entry = entry;
        entry.vector = vector.clone();

        if let Some(expected_dim) = collection.manifest.dimensions {
            piramid_core::validation::validate_dimensions(&vector, expected_dim)?;
        } else {
            collection.manifest.set_dimensions(vector.len())?;
        }

        let bytes = RecordStore::encode_document(&entry)?;
        limits::enforce_single(collection, bytes.len())?;

        let index_entry = collection.record_store.append(&bytes)?;
        collection.index.insert(*id, index_entry);
        collection.cache.put_vector(*id, &vector)?;
        collection.cache.put_metadata(*id, entry.metadata.clone());
        collection.vector_index.remove(id);
        collection
            .vector_index
            .insert(*id, &vector, &collection.cache)?;
        collection
            .manifest
            .update_vector_count(collection.index.len());
        collection.track_operation()?;
        Ok(true)
    } else {
        Ok(false)
    }
}
