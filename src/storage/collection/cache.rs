use crate::storage::collection::operations;
use crate::storage::collection::storage::Collection;

pub fn rebuild(collection: &mut Collection) {
    collection.vector_cache.clear();
    for (id, _) in &collection.index {
        if let Some(entry) = operations::get(collection, id) {
            collection.vector_cache.insert(*id, entry.get_vector());
        }
    }
}

pub fn ensure_consistent(collection: &mut Collection) {
    if collection.vector_cache.len() != collection.index.len() {
        rebuild(collection);
        return;
    }
    for (id, _) in &collection.index {
        if !collection.vector_cache.contains_key(id) {
            rebuild(collection);
            break;
        }
    }
}
