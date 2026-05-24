use uuid::Uuid;

use super::super::collection::Collection;
use super::limits;
use super::read::get;
use crate::error::Result;
use crate::metadata::Metadata;
use crate::quantization::QuantizedVector;
use crate::storage::record_store::RecordStore;
use crate::storage::wal::WalEntry;

pub fn update_metadata(storage: &mut Collection, id: &Uuid, metadata: Metadata) -> Result<bool> {
    if let Some(entry) = get(storage, id) {
        let mut wal_entry = WalEntry::Update {
            id: *id,
            vector: entry.get_vector(),
            text: entry.text.clone(),
            metadata: metadata.clone(),
            seq: 0,
        };
        storage.checkpoint.wal.log(&mut wal_entry)?;

        let mut entry = entry;
        entry.metadata = metadata.clone();
        let bytes = RecordStore::encode_document(&entry)?;

        limits::enforce_single(storage, bytes.len())?;
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
        storage.checkpoint.wal.log(&mut wal_entry)?;

        let mut entry = entry;
        entry.vector = QuantizedVector::from_f32_with_config(&vector, &storage.config.quantization);

        if let Some(expected_dim) = storage.metadata.dimensions {
            crate::validation::validate_dimensions(&vector, expected_dim)?;
        } else {
            storage.metadata.set_dimensions(vector.len());
        }

        let bytes = RecordStore::encode_document(&entry)?;
        limits::enforce_single(storage, bytes.len())?;

        let index_entry = storage.record_store.append(&bytes)?;
        storage.index.insert(*id, index_entry);
        storage.cache.put_vector(*id, vector.clone());
        storage.cache.put_metadata(*id, entry.metadata.clone());
        storage.vector_index.remove(id);
        storage.vector_index.insert(*id, &vector, &storage.cache);
        storage.metadata.update_vector_count(storage.index.len());
        storage.track_operation()?;
        Ok(true)
    } else {
        Ok(false)
    }
}
