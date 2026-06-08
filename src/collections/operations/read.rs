use uuid::Uuid;

use super::super::collection::Collection;
use crate::error::Result;
use crate::storage::document::Document;

pub fn get(storage: &Collection, id: &Uuid) -> Result<Option<Document>> {
    let Some(index_entry) = storage.index.get(id) else {
        return Ok(None);
    };
    storage.record_store.read_document(index_entry).map(Some)
}
