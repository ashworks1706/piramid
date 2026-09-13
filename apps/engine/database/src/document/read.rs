use uuid::Uuid;

use super::super::collection::Collection;
use piramid_core::error::Result;
use piramid_core::Document;

pub fn get(collection: &Collection, id: &Uuid) -> Result<Option<Document>> {
    let Some(index_entry) = collection.offsets.get(id) else {
        return Ok(None);
    };
    collection.record_store.read_document(index_entry).map(Some)
}
