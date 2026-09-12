use uuid::Uuid;

use super::super::collection::Collection;
use super::read::get;
use crate::collection::limits;
use crate::storage::record_store::RecordStore;
use crate::storage::wal::WalEntry;
use piramid_core::error::Result;
use piramid_core::metadata::Metadata;

pub fn update_metadata(collection: &mut Collection, id: &Uuid, metadata: Metadata) -> Result<bool> {
    let Some(mut entry) = get(collection, id)? else {
        return Ok(false);
    };
    entry.metadata = metadata;
    let bytes = RecordStore::encode_document(&entry)?;
    limits::enforce_single(collection, bytes.len(), true)?;

    let mut wal_entry = WalEntry::Update {
        id: *id,
        vector: entry.vector().to_vec(),
        text: entry.text.clone(),
        metadata: entry.metadata.clone(),
        seq: 0,
    };
    collection.checkpoint.wal.log(&mut wal_entry)?;

    let pointer = collection.record_store.append(&bytes)?;
    collection.index.insert(*id, pointer);
    collection.cache.put_metadata(*id, entry.metadata);
    collection.track_operation()?;
    Ok(true)
}
