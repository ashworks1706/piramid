use uuid::Uuid;

use super::super::collection::Collection;
use crate::storage::document::Document;

pub fn get(storage: &Collection, id: &Uuid) -> Option<Document> {
    let index_entry = storage.index.get(id)?;
    storage.record_store.read_document(index_entry)
}
