// Compaction logic for collections, including rewriting live documents and rebuilding indexes.
//  takes a mutable reference to a `Collection` and performs compaction by creating a new temporary file, copying live documents to it, rebuilding the index and vector index, and then replacing the original file with the compacted version.
use std::collections::HashMap;

use super::record_store::RecordStore;
use super::storage::Collection;
use crate::error::Result;
use crate::storage::document::Document;
use crate::storage::persistence::{save_index, save_metadata, save_vector_index};

/// Compact a collection by rewriting live documents into a fresh file and rebuilding indexes.
pub fn compact(collection: &mut Collection) -> Result<CompactStats> {
    // 1. Get all live documents and their count before compaction
    let original_entries = collection.index.len();
    let docs: Vec<Document> = collection.get_all();

    let temp_path = format!("{}.compact", collection.path);
    let _ = std::fs::remove_file(&temp_path);
    let mut temp_store = RecordStore::open(&temp_path, &collection.config, &HashMap::new())?;
    let mut new_index = HashMap::with_capacity(docs.len());
    let mut new_vectors = HashMap::with_capacity(docs.len());
    let mut new_vector_index = collection.config.index.create_index(docs.len());
    let mut new_metadata = collection.metadata.clone();
    new_metadata.update_vector_count(0);

    for doc in docs {
        let id = doc.id;
        let vector = doc.get_vector();
        let bytes = RecordStore::encode_document(&doc)?;
        let pointer = temp_store.append(&bytes)?;
        new_metadata.set_dimensions(vector.len());
        new_index.insert(id, pointer);
        new_vectors.insert(id, vector.clone());
        new_vector_index.insert(id, &vector, &new_vectors);
    }
    new_metadata.update_vector_count(new_index.len());

    temp_store.sync()?;
    drop(temp_store);
    std::fs::rename(&temp_path, &collection.path)?;

    collection.record_store = RecordStore::open(&collection.path, &collection.config, &new_index)?;
    collection.index = new_index;
    collection.vector_index = new_vector_index;
    collection.metadata = new_metadata;
    collection.clear_caches_for_rebuild();
    collection.rebuild_vector_cache();

    // 4. Save the new index, vector index, and metadata to disk after compaction
    save_index(&collection.path, &collection.index)?;
    save_vector_index(&collection.path, collection.vector_index())?;
    save_metadata(&collection.path, &collection.metadata)?;
    // Rotate WAL to drop old entries after compaction
    let _ = collection.persistence.wal.rotate();

    Ok(CompactStats {
        original_entries,
        compacted_entries: collection.index.len(),
    })
}

#[derive(Debug)]
pub struct CompactStats {
    pub original_entries: usize,
    pub compacted_entries: usize,
}
