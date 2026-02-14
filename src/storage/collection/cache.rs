// Maintains the in-memory, dequantized vector cache for a collection.
// The vector cache is used to speed up search operations by keeping the dequantized vectors in memory, allowing for faster access during similarity search. The cache is kept in sync with the main index and metadata, and can be rebuilt if inconsistencies are detected. This module provides functions to rebuild the cache from the main index and to ensure that the cache remains consistent with the underlying data.
use crate::storage::collection::operations;
use crate::storage::collection::storage::Collection;
pub fn rebuild(collection: &mut Collection) {
    // Clear the existing caches before rebuilding to ensure that we start with a clean state. This is important because if there are inconsistencies between the cache and the main index, we want to make sure that we remove any stale entries from the cache before repopulating it with the correct data from the index. By clearing the caches first, we can avoid potential issues with outdated or incorrect data being retained in the cache during the rebuild process.
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
    // Check if the vector cache is consistent with the main index. If the number of entries in the vector cache does not match the number of entries in the index, we know that there is an inconsistency and we need to rebuild the cache. This is a quick check to detect any discrepancies between the cache and the index, which can occur due to various reasons such as failed updates, crashes, or bugs in the code. If we detect an inconsistency, we call the rebuild function to repopulate the cache with the correct data from the index.
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
