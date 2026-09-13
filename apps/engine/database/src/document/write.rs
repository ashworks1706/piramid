use uuid::Uuid;

use super::super::collection::Collection;
use super::read::get;
use crate::collection::limits;
use crate::storage::record_store::RecordStore;
use crate::storage::wal::WalEntry;
use piramid_core::error::{Result, ServerError};
use piramid_core::Document;
use piramid_hardware::compute::Metric;

/// Check the width of vector against the width the collection holds, touching nothing.
fn check_width(collection: &Collection, vector: &[f32]) -> Result<()> {
    match collection.manifest.dimensions {
        Some(expected) => piramid_core::validation::validate_dimensions(vector, expected),
        None => Ok(()),
    }
}

/// Reject a vector the metric of the collection cannot score, touching nothing.
fn check_scorable(collection: &Collection, vector: &[f32]) -> Result<()> {
    match collection.metric() {
        Metric::Cosine => piramid_core::validation::validate_cosine_magnitude(vector),
        Metric::Euclidean | Metric::DotProduct => Ok(()),
    }
}

/// Validate and encode one document, touching nothing. replacing is true when the document takes
/// the place of one already stored, which does not grow the vector count.
fn prepare(collection: &Collection, entry: &Document, replacing: bool) -> Result<Vec<u8>> {
    check_width(collection, entry.vector())?;
    let bytes = RecordStore::encode_document(entry)?;
    limits::enforce_single(collection, bytes.len(), replacing)?;
    Ok(bytes)
}

/// Store an encoded document that has passed [prepare], and make it resident.
fn apply_insert(collection: &mut Collection, entry: Document, bytes: &[u8]) -> Result<Uuid> {
    let id = entry.id;
    let pointer = collection.record_store.append(bytes)?;
    collection.offsets.insert(id, pointer);
    collection.manifest.set_dimensions(entry.vector().len())?;
    collection.resident.put_vector(id, entry.vector())?;
    collection.resident.put_metadata(id, entry.metadata);
    collection
        .manifest
        .update_vector_count(collection.offsets.len())?;
    Ok(id)
}

/// Validate and store one document without logging it. Used by WAL replay.
pub fn insert_internal(collection: &mut Collection, entry: Document) -> Result<Uuid> {
    let replacing = collection.offsets.contains_key(&entry.id);
    let bytes = prepare(collection, &entry, replacing)?;
    apply_insert(collection, entry, &bytes)
}

pub fn delete_internal(collection: &mut Collection, id: &Uuid) -> Result<()> {
    collection.offsets.remove(id);
    collection.resident.remove(id);
    collection
        .manifest
        .update_vector_count(collection.offsets.len())
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

/// Refuse an id already stored, touching nothing.
fn check_new_id(collection: &Collection, id: &Uuid) -> Result<()> {
    if collection.offsets.contains_key(id) {
        return Err(ServerError::InvalidRequest(format!(
            "document {id} already exists; use upsert"
        ))
        .into());
    }
    Ok(())
}

pub fn insert(collection: &mut Collection, entry: Document) -> Result<Uuid> {
    collection.ensure_writable()?;
    check_new_id(collection, &entry.id)?;
    check_scorable(collection, entry.vector())?;
    let bytes = prepare(collection, &entry, false)?;
    let mut wal_entry = insert_wal_entry(&entry);
    collection.checkpoint.wal.log(&mut wal_entry)?;

    let id = apply_insert(collection, entry, &bytes)?;
    collection.track_operation()?;
    Ok(id)
}

pub fn insert_batch(collection: &mut Collection, entries: Vec<Document>) -> Result<Vec<Uuid>> {
    collection.ensure_writable()?;
    let Some(first) = entries.first() else {
        return Ok(Vec::new());
    };
    let width = collection
        .manifest
        .dimensions
        .unwrap_or(first.vector().len());
    let mut serialized: Vec<(Uuid, Vec<u8>)> = Vec::with_capacity(entries.len());
    let mut batch_ids = std::collections::HashSet::with_capacity(entries.len());
    for entry in &entries {
        check_new_id(collection, &entry.id)?;
        if !batch_ids.insert(entry.id) {
            return Err(ServerError::InvalidRequest(format!(
                "document {} appears more than once in the batch",
                entry.id
            ))
            .into());
        }
        piramid_core::validation::validate_dimensions(entry.vector(), width)?;
        check_scorable(collection, entry.vector())?;
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
        collection.offsets.insert(id, pointer);
        collection.resident.put_vector(id, entry.vector())?;
        collection.resident.put_metadata(id, entry.metadata);
        ids.push(id);
    }
    collection
        .manifest
        .update_vector_count(collection.offsets.len())?;
    collection.track_operation()?;

    Ok(ids)
}

pub fn upsert(collection: &mut Collection, entry: Document) -> Result<Uuid> {
    collection.ensure_writable()?;
    let id = entry.id;
    if !collection.offsets.contains_key(&id) {
        return insert(collection, entry);
    }
    check_scorable(collection, entry.vector())?;
    let bytes = prepare(collection, &entry, true)?;

    let mut wal_entry = WalEntry::Update {
        id,
        vector: entry.vector().to_vec(),
        text: entry.text.clone(),
        metadata: entry.metadata.clone(),
        seq: 0,
    };
    collection.checkpoint.wal.log(&mut wal_entry)?;

    delete_internal(collection, &id)?;
    apply_insert(collection, entry, &bytes)?;
    collection.track_operation()?;
    Ok(id)
}

pub fn delete(collection: &mut Collection, id: &Uuid) -> Result<bool> {
    collection.ensure_writable()?;
    if collection.offsets.contains_key(id) {
        let mut wal_entry = WalEntry::Delete { id: *id, seq: 0 };
        collection.checkpoint.wal.log(&mut wal_entry)?;

        delete_internal(collection, id)?;
        collection.track_operation()?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn delete_batch(collection: &mut Collection, ids: &[Uuid]) -> Result<usize> {
    collection.ensure_writable()?;
    let mut deleted_count = 0;

    for id in ids {
        if collection.offsets.contains_key(id) {
            let mut wal_entry = WalEntry::Delete { id: *id, seq: 0 };
            collection.checkpoint.wal.log(&mut wal_entry)?;
        }
    }

    for id in ids {
        if collection.offsets.contains_key(id) {
            delete_internal(collection, id)?;
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
