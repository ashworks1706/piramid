use uuid::Uuid;

use super::super::collection::Collection;
use super::read::get;
use crate::collection::limits;
use crate::storage::record_store::RecordStore;
use crate::storage::wal::WalEntry;
use piramid_core::error::Result;
use piramid_core::Document;

/// Check the width of vector against the width the collection holds, touching nothing.
fn check_width(collection: &Collection, vector: &[f32]) -> Result<()> {
    match collection.manifest.dimensions {
        Some(expected) => piramid_core::validation::validate_dimensions(vector, expected),
        None => Ok(()),
    }
}

/// Validate and encode one document, touching nothing. replacing is true when the document takes
/// the place of one already stored, so the vector count does not grow.
fn prepare(collection: &Collection, entry: &Document, replacing: bool) -> Result<Vec<u8>> {
    check_width(collection, entry.vector())?;
    let bytes = RecordStore::encode_document(entry)?;
    limits::enforce_single(collection, bytes.len(), replacing)?;
    Ok(bytes)
}

/// Store an encoded document that has passed [prepare], and index it.
fn apply_insert(collection: &mut Collection, entry: Document, bytes: &[u8]) -> Result<Uuid> {
    let id = entry.id;
    let pointer = collection.record_store.append(bytes)?;
    collection.index.insert(id, pointer);
    collection.manifest.set_dimensions(entry.vector().len())?;
    collection.cache.put_vector(id, entry.vector())?;
    collection
        .vector_index
        .insert(id, entry.vector(), &collection.cache)?;
    collection.cache.put_metadata(id, entry.metadata);
    collection
        .manifest
        .update_vector_count(collection.index.len());
    collection.grow_index_family()?;
    Ok(id)
}

/// Validate, store and index one document without logging it. Used by WAL replay.
pub fn insert_internal(collection: &mut Collection, entry: Document) -> Result<Uuid> {
    let replacing = collection.index.contains_key(&entry.id);
    let bytes = prepare(collection, &entry, replacing)?;
    apply_insert(collection, entry, &bytes)
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
    let bytes = prepare(collection, &entry, false)?;
    let mut wal_entry = insert_wal_entry(&entry);
    collection.checkpoint.wal.log(&mut wal_entry)?;

    let id = apply_insert(collection, entry, &bytes)?;
    collection.track_operation()?;
    Ok(id)
}

pub fn insert_batch(collection: &mut Collection, entries: Vec<Document>) -> Result<Vec<Uuid>> {
    let Some(first) = entries.first() else {
        return Ok(Vec::new());
    };
    let width = collection
        .manifest
        .dimensions
        .unwrap_or(first.vector().len());
    let mut serialized: Vec<(Uuid, Vec<u8>)> = Vec::with_capacity(entries.len());
    for entry in &entries {
        piramid_core::validation::validate_dimensions(entry.vector(), width)?;
        serialized.push((entry.id, RecordStore::encode_document(entry)?));
    }
    let total_bytes: u64 = serialized.iter().map(|(_, bytes)| bytes.len() as u64).sum();
    let max_entry_bytes = serialized.iter().map(|(_, bytes)| bytes.len()).max();
    limits::enforce_batch(collection, serialized.len(), total_bytes, max_entry_bytes)?;

    for entry in &entries {
        let mut wal_entry = insert_wal_entry(entry);
        collection.checkpoint.wal.log(&mut wal_entry)?;
    }

    let pointers = collection.record_store.append_batch(&serialized)?;
    let mut ids = Vec::with_capacity(entries.len());
    collection.manifest.set_dimensions(width)?;
    for (entry, pointer) in entries.into_iter().zip(pointers) {
        let id = entry.id;
        collection.index.insert(id, pointer);
        collection.cache.put_vector(id, entry.vector())?;
        collection
            .vector_index
            .insert(id, entry.vector(), &collection.cache)?;
        collection.cache.put_metadata(id, entry.metadata);
        ids.push(id);
    }
    collection
        .manifest
        .update_vector_count(collection.index.len());
    collection.grow_index_family()?;
    collection.track_operation()?;

    Ok(ids)
}

pub fn upsert(collection: &mut Collection, entry: Document) -> Result<Uuid> {
    let id = entry.id;
    if !collection.index.contains_key(&id) {
        return insert(collection, entry);
    }
    let bytes = prepare(collection, &entry, true)?;

    let mut wal_entry = WalEntry::Update {
        id,
        vector: entry.vector().to_vec(),
        text: entry.text.clone(),
        metadata: entry.metadata.clone(),
        seq: 0,
    };
    collection.checkpoint.wal.log(&mut wal_entry)?;

    delete_internal(collection, &id);
    apply_insert(collection, entry, &bytes)?;
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
    let Some(mut entry) = get(collection, id)? else {
        return Ok(false);
    };
    entry.vector = vector;
    upsert(collection, entry)?;
    Ok(true)
}
