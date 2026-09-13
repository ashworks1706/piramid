//! Compaction: rewrite the record store without dead entries.

use std::collections::HashMap;

use super::Collection;
use crate::index::save_vector_index;
use crate::index::HashMapVectorReader;
use crate::storage::record_store::RecordStore;
use crate::storage::SidecarManager;
use piramid_core::error::Result;
use piramid_core::Document;

/// Compact a collection by rewriting live documents into a fresh file and rebuilding indexes.
pub fn compact(collection: &mut Collection) -> Result<CompactStats> {
    let bytes_before = collection.record_store.used_bytes();
    let docs: Vec<Document> = collection.get_all()?;

    let temp_path = SidecarManager::at(&collection.path).compact_path();
    match std::fs::remove_file(&temp_path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let mut temp_store = RecordStore::open(&temp_path, &collection.config, &HashMap::new())?;
    let mut new_index = HashMap::with_capacity(docs.len());
    let mut new_vectors = HashMap::with_capacity(docs.len());
    let mut new_vector_index = crate::index::create_index(
        &collection.config.index,
        collection.config.execution,
        docs.len(),
    );
    let mut new_manifest = collection.manifest.clone();
    new_manifest.update_vector_count(0)?;

    for doc in docs {
        let id = doc.id;
        let vector = doc.vector().to_vec();
        let bytes = RecordStore::encode_document(&doc)?;
        let pointer = temp_store.append(&bytes)?;
        new_manifest.set_dimensions(vector.len())?;
        new_index.insert(id, pointer);
        new_vectors.insert(id, vector.clone());
        let reader = HashMapVectorReader::new(&new_vectors);
        new_vector_index.insert(id, &vector, &reader)?;
    }
    new_manifest.update_vector_count(new_index.len())?;

    temp_store.sync()?;
    drop(temp_store);
    std::fs::rename(&temp_path, &collection.path)?;

    collection.record_store = RecordStore::open(&collection.path, &collection.config, &new_index)?;
    collection.index = new_index;
    collection.vector_index = new_vector_index;
    collection.manifest = new_manifest;
    collection.clear_caches_for_rebuild();
    collection.rebuild_vector_cache()?;

    let sidecars = SidecarManager::at(&collection.path);
    sidecars.save_manifest(&collection.manifest)?;
    sidecars.save_offsets(&collection.index)?;
    save_vector_index(&collection.path, collection.vector_index())?;
    // Sidecars are durable before the WAL entries they made redundant are dropped.
    collection.checkpoint.wal.rotate()?;

    Ok(CompactStats {
        documents: collection.index.len(),
        bytes_before,
        bytes_after: collection.record_store.used_bytes(),
    })
}

/// What a compaction kept and reclaimed.
#[derive(Debug)]
pub struct CompactStats {
    /// Live documents rewritten into the new record file.
    pub documents: usize,
    /// Bytes of records in the data file before compaction, dead entries included.
    pub bytes_before: u64,
    /// Bytes of records in the data file after compaction.
    pub bytes_after: u64,
}
