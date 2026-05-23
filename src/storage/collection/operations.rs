// Collection CRUD operations.
use uuid::Uuid;

use super::record_store::RecordStore;
use super::storage::Collection;
use crate::error::{Result, ServerError};
use crate::index::HashMapVectorReader;
use crate::metadata::Metadata;
use crate::quantization::QuantizedVector;
use crate::storage::document::Document;
use crate::storage::wal::WalEntry;

fn enforce_limits_single(storage: &Collection, entry_bytes: usize) -> Result<()> {
    let limits = storage.config.limits;

    if let Some(max_vecs) = limits.max_vectors {
        if storage.count() >= max_vecs {
            return Err(
                ServerError::InvalidRequest("Collection max vectors reached".into()).into(),
            );
        }
    }

    if let Some(max_bytes) = limits.max_bytes {
        let current_size = storage.record_store.used_bytes();
        let required = current_size.saturating_add(entry_bytes as u64);
        if required > max_bytes {
            return Err(ServerError::InvalidRequest("Collection max size reached".into()).into());
        }
    }

    if let Some(max_vec_bytes) = limits.max_vector_bytes {
        if entry_bytes > max_vec_bytes {
            return Err(
                ServerError::InvalidRequest("Vector exceeds max allowed size".into()).into(),
            );
        }
    }

    Ok(())
}

// current + batch <= max
fn enforce_limits_batch(
    storage: &Collection,
    total_entries: usize,
    total_bytes: u64,
    max_entry_bytes: Option<usize>,
) -> Result<()> {
    let limits = storage.config.limits;

    if let Some(max_vecs) = limits.max_vectors {
        let current = storage.count();
        if current.saturating_add(total_entries) > max_vecs {
            return Err(
                ServerError::InvalidRequest("Collection max vectors reached".into()).into(),
            );
        }
    }

    if let Some(max_bytes) = limits.max_bytes {
        let current_size = storage.record_store.used_bytes();
        let required = current_size.saturating_add(total_bytes);
        if required > max_bytes {
            return Err(ServerError::InvalidRequest("Collection max size reached".into()).into());
        }
    }

    if let Some(max_vec_bytes) = max_entry_bytes {
        if max_vec_bytes > 0 {
            if let Some(cfg_limit) = limits.max_vector_bytes {
                if max_vec_bytes > cfg_limit {
                    return Err(ServerError::InvalidRequest(
                        "Vector exceeds max allowed size".into(),
                    )
                    .into());
                }
            }
        }
    }

    Ok(())
}

pub fn get(storage: &Collection, id: &Uuid) -> Option<Document> {
    let index_entry = storage.index.get(id)?;
    storage.record_store.read_document(index_entry)
}

pub fn insert_internal(storage: &mut Collection, mut entry: Document) -> Result<Uuid> {
    let id = entry.id;
    let raw_vec = entry.get_vector();
    entry.vector = QuantizedVector::from_f32_with_config(&raw_vec, &storage.config.quantization);
    let bytes = RecordStore::encode_document(&entry)?;

    enforce_limits_single(storage, bytes.len())?;
    let index_entry = storage.record_store.append(&bytes)?;
    storage.index.insert(id, index_entry.clone());

    storage.metadata.set_dimensions(raw_vec.len());

    // Keep the collection dimension aligned with the first inserted vector.
    if let Some(expected_dim) = storage.metadata.dimensions {
        crate::validation::validate_dimensions(&raw_vec, expected_dim)?;
    }

    storage.cache.put_vector(id, raw_vec.clone());
    storage.cache.put_metadata(id, entry.metadata.clone());
    let vectors = HashMapVectorReader::new(storage.cache.vectors());
    storage.vector_index.insert(id, &raw_vec, &vectors);

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
    storage.persistence.wal.log(&mut wal_entry)?;

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
        // Log each insert before applying the batch.
        storage.persistence.wal.log(&mut wal_entry)?;
    }

    let mut serialized: Vec<(Uuid, Vec<u8>)> = Vec::with_capacity(entries.len());
    let mut raw_vectors: Vec<(Uuid, Vec<f32>, Metadata)> = Vec::with_capacity(entries.len());
    for entry in &mut entries {
        let raw_vec = entry.get_vector();
        let metadata = entry.metadata.clone();
        entry.vector =
            QuantizedVector::from_f32_with_config(&raw_vec, &storage.config.quantization);
        let bytes = RecordStore::encode_document(entry)?;
        serialized.push((entry.id, bytes));
        raw_vectors.push((entry.id, raw_vec, metadata));
    }
    let total_bytes: u64 = serialized.iter().map(|(_, b)| b.len() as u64).sum();
    let max_entry_bytes = serialized.iter().map(|(_, b)| b.len()).max();
    enforce_limits_batch(storage, serialized.len(), total_bytes, max_entry_bytes)?;
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
        let vectors = HashMapVectorReader::new(storage.cache.vectors());
        storage.vector_index.insert(id, &vec_f32, &vectors);
    }
    storage.metadata.update_vector_count(storage.index.len());

    Ok(ids)
}

pub fn upsert(storage: &mut Collection, mut entry: Document) -> Result<Uuid> {
    let id = entry.id;
    let raw_vec = entry.get_vector();
    entry.vector = QuantizedVector::from_f32_with_config(&raw_vec, &storage.config.quantization);
    let bytes = RecordStore::encode_document(&entry)?;

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
        insert_internal(storage, entry)?;
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
        let bytes = RecordStore::encode_document(&entry)?;

        enforce_limits_single(storage, bytes.len())?;
        let index_entry = storage.record_store.append(&bytes)?;
        storage.index.insert(*id, index_entry);
        storage.cache.put_metadata(*id, metadata);
        storage.metadata.update_vector_count(storage.index.len());
        storage.track_operation()?;
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

        let bytes = RecordStore::encode_document(&entry)?;
        enforce_limits_single(storage, bytes.len())?;

        let index_entry = storage.record_store.append(&bytes)?;
        storage.index.insert(*id, index_entry);
        storage.cache.put_vector(*id, vector.clone());
        storage.cache.put_metadata(*id, entry.metadata.clone());
        storage.vector_index.remove(id);
        let vectors = HashMapVectorReader::new(storage.cache.vectors());
        storage.vector_index.insert(*id, &vector, &vectors);
        storage.metadata.update_vector_count(storage.index.len());
        storage.track_operation()?;
        Ok(true)
    } else {
        Ok(false)
    }
}
