// Compaction logic for collections, including rewriting live documents and rebuilding indexes.
// This module defines the `compact` function, which takes a mutable reference to a `Collection` and performs compaction by creating a new temporary file, copying live documents to it, rebuilding the index and vector index, and then replacing the original file with the compacted version. It also defines a `CompactStats` struct to report the results of the compaction process.
use crate::error::Result;
use crate::storage::document::Document;
use crate::storage::persistence::{save_index, save_vector_index, save_metadata, create_mmap, ensure_file_size};
use super::storage::Collection;
use crate::storage::collection::operations;

/// Compact a collection by rewriting live documents into a fresh file and rebuilding indexes.
pub fn compact(collection: &mut Collection) -> Result<CompactStats> {

    // 1. Get all live documents and their count before compaction
    let original_entries = collection.index.len();
    let docs: Vec<Document> = collection.get_all();

    // Reset file
    drop(collection.mmap.take());
    let initial_size = if collection.config.memory.use_mmap {
        collection.config.memory.initial_mmap_size as u64
    } else {
        1024 * 1024
    };
    // 2. Truncate the existing data file and prepare for rewriting
    collection.data_file.set_len(0)?;
    ensure_file_size(&collection.data_file, initial_size)?;
    collection.mmap = if collection.config.memory.use_mmap {
        Some(create_mmap(&collection.data_file)?)
    } else {
        None
    };


    // 3. Clear existing indexes and caches in preparation for rebuilding
    // Reset indexes and caches
    collection.index.clear();
    collection.vector_index = collection.config.index.create_index(0);
    collection.vector_cache.clear();
    collection.metadata_cache.clear();
    collection.metadata.update_vector_count(0);

    // Reinsert all documents
    for doc in docs {
        operations::insert_internal(collection, doc)?;
    }


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
