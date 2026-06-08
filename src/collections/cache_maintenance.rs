use crate::cache::CacheManager;
use crate::Result;

use super::collection::Collection;
use super::operations;

pub fn rebuild(collection: &mut Collection) -> Result<()> {
    let mut cache = CacheManager::new(collection.config.cache);
    for id in collection.index.keys() {
        if let Some(entry) = operations::get(collection, id)? {
            cache.put_vector(*id, entry.try_get_vector()?);
            cache.put_metadata(*id, entry.metadata.clone());
        }
    }
    collection.cache = cache;
    Ok(())
}

pub fn ensure_consistent(collection: &mut Collection) -> Result<()> {
    if collection.cache.vector_len() != collection.index.len() {
        rebuild(collection)?;
        return Ok(());
    }

    for id in collection.index.keys() {
        if !collection.cache.vector_contains(id) {
            rebuild(collection)?;
            break;
        }
    }
    Ok(())
}
