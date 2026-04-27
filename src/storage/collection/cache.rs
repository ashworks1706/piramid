// Maintains the in-memory, dequantized vector cache for a collection.
//  speed up by keeping the dequantized vectors in memory, kept in sync with the main index and metadata, and can be rebuilt if inconsistencies are detected. rebuild the cache from the main index and to ensure that the cache remains consistent with the underlying data.
use crate::storage::collection::operations;
use crate::storage::collection::storage::Collection;
pub fn rebuild(collection: &mut Collection) {
    // Clear the existing caches before rebuilding to ensure that we start with a clean state.
    collection.vector_cache.clear();
    collection.metadata_cache.clear();
    for (id, _) in &collection.index {
        if let Some(entry) = operations::get(collection, id) {
            collection.vector_cache.insert(*id, entry.get_vector());
            collection.metadata_cache.insert(*id, entry.metadata.clone());
        }
    }
}

pub fn ensure_consistent(collection: &mut Collection) {
    //  If the number of entries in the vector cache does not match the number of entries in the index, we know that there is an inconsistency and we need to rebuild the cache.
    if collection.vector_cache.len() != collection.index.len() {
        rebuild(collection);
        return;
    }
    for (id, _) in &collection.index {
        if !collection.vector_cache.contains_key(id) {
            rebuild(collection);
            break;
        }
        if !collection.metadata_cache.contains_key(id) {
            rebuild(collection);
            break;
        }
    }
}
