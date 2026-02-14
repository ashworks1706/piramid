// Collection CRUD operations
// This module implements the core CRUD operations for the collection, including get, insert, delete, and update. These operations interact with the underlying storage layer to read and write documents, update the index and vector index, and manage the in-memory caches. The insert and delete operations also log changes to the WAL for durability and recovery purposes. The update operations allow for modifying either the metadata or the vector of an existing document while ensuring that the changes are properly persisted and reflected in the index and caches.
use uuid::Uuid;

use crate::error::Result;
use crate::storage::document::Document;
use crate::storage::wal::WalEntry;
use crate::storage::persistence::{EntryPointer, grow_mmap_if_needed};
use crate::quantization::QuantizedVector;
use crate::metadata::Metadata;
use super::storage::Collection;

pub fn get(storage: &Collection, id: &Uuid) -> Option<Document> {
    let index_entry = storage.index.get(id)?;
    let offset = index_entry.offset as usize;
    let length = index_entry.length as usize;
    let bytes = &storage.mmap.as_ref().unwrap()[offset..offset + length];
    bincode::deserialize(bytes).ok()
}

pub fn insert_internal(storage: &mut Collection, entry: Document) -> Result<Uuid> {
    // 1. Serialize the document entry into bytes using bincode. This will allow us to write the document data to the memory-mapped file in a compact binary format. The serialized bytes will include all the necessary information about the document, such as its ID, vector, text, and metadata.
    let id = entry.id;
    let bytes = bincode::serialize(&entry)?; 

    // 2. Calculate the offset for where to write the new document in the memory-mapped file. We find the maximum offset of existing entries in the index and add the length of those entries to determine where the new entry should be written. This ensures that we append new entries to the end of the file without overwriting existing data.
    let offset = storage.index.values()
        .map(|idx| idx.offset + idx.length as u64)
        .max()
        .unwrap_or(0);

    // 3. Check if we need to grow the memory-mapped file to accommodate the new entry. If the required size (offset + length of new entry) exceeds the current size of the memory-mapped file, we need to grow it. This involves unmapping the current memory map, resizing the underlying file, and creating a new memory map with the updated size. By growing the memory-mapped file as needed, we can ensure that we have enough space to write new entries without running into out-of-bounds errors.
    let required_size = offset + bytes.len() as u64;
    grow_mmap_if_needed(&mut storage.mmap, &storage.data_file, required_size)?;
    

    // 4. Write the serialized bytes of the document to the memory-mapped file at the calculated offset. We use the memory map to directly write the bytes to the file, which allows for efficient I/O operations. After writing the bytes, we create an index entry that records the offset and length of the new document in the file, and we insert this entry into the main index of the collection. This will allow us to quickly locate and retrieve the document in future get operations.
    let mmap = storage.mmap.as_mut().unwrap();
    mmap[offset as usize..(offset as usize + bytes.len())]
        .copy_from_slice(&bytes);
    
    // 5. Update the vector index and cache with the new document's vector. We extract the vector from the document, update the metadata with the dimensions of the vector, and then insert the vector into the in-memory cache and the vector index. This ensures that the new document is included in future search operations and that its vector is readily available for similarity calculations.
    let index_entry = EntryPointer::new(offset, bytes.len() as u32);
    storage.index.insert(id, index_entry);
    
    let vec_f32 = entry.get_vector();
    
    // Update the collection metadata with the dimensions of the new vector. This is important for ensuring that all vectors in the collection have consistent dimensions, which is a requirement for similarity search. If the collection already has a defined dimension, we validate that the new vector matches that dimension. If the collection does not have a defined dimension yet, we set it based on the first inserted vector.
    storage.metadata.set_dimensions(vec_f32.len());
    
    // Validate that the dimensions of the new vector match the expected dimensions of the collection. If the collection has a defined dimension, we check that the length of the new vector matches that dimension. If there is a mismatch, we return an error to prevent inserting inconsistent data into the collection. This validation step helps maintain the integrity of the collection and ensures that all vectors are compatible for similarity search operations.
    if let Some(expected_dim) = storage.metadata.dimensions {
        crate::validation::validate_dimensions(&vec_f32, expected_dim)?;
    }
    
    // Insert the new vector into the in-memory cache and the vector index. This allows for fast access to the vector during search operations without needing to read from the memory-mapped file. By keeping the vector cache and index updated with new entries, we can ensure that search operations remain efficient and that the collection is ready to handle queries immediately after insertion.
    storage.vector_cache.insert(id, vec_f32.clone());
    storage.vector_index.insert(id, &vec_f32, &storage.vector_cache);
    
    storage.metadata.update_vector_count(storage.index.len());
    
    Ok(id)
}

pub fn delete_internal(storage: &mut Collection, id: &Uuid) {
    storage.index.remove(id);
    storage.vector_index.remove(id);
    storage.vector_cache.remove(id);
    storage.metadata.update_vector_count(storage.index.len());
}

pub fn insert(storage: &mut Collection, entry: Document) -> Result<Uuid> {
    let vector = entry.get_vector();
    let mut wal_entry = WalEntry::Insert { 
        id: entry.id, 
        vector,
        text: entry.text.clone(),
        metadata: entry.metadata.clone(),
        seq: 0,
    };
    storage.persistence.wal.log(&mut wal_entry)?;
    
    super::persistence::save_index(storage)?;
    storage.track_operation()?;

    insert_internal(storage, entry)
}

pub fn insert_batch(storage: &mut Collection, entries: Vec<Document>) -> Result<Vec<Uuid>> {
    // Log all the entries to the WAL before inserting them into the collection. This ensures that we have a record of all the operations in the WAL for durability and recovery purposes. By logging the entries first, we can guarantee that even if there is a failure during the insertion process, we can recover the intended state of the collection by replaying the WAL entries.
    let mut ids = Vec::with_capacity(entries.len());
    
    //  Iterate through each entry and log it to the WAL. For each entry, we create a corresponding WAL entry with the necessary information (ID, vector, text, metadata) and log it using the WAL instance. This allows us to maintain a complete history of all insert operations, which is crucial for ensuring durability and enabling recovery in case of crashes or unexpected shutdowns.
    for entry in &entries {
        let vector = entry.get_vector();
        let mut wal_entry = WalEntry::Insert {
            id: entry.id,
            vector,
            text: entry.text.clone(),
            metadata: entry.metadata.clone(),
            seq: 0,
        };
        storage.persistence.wal.log(&mut wal_entry)?;
    }
    // After logging all entries to the WAL, we proceed to insert them into the collection. This involves serializing each entry, writing it to the memory-mapped file, updating the index and vector index, and updating the in-memory caches. By separating the logging and insertion steps, we can ensure that we have a clear record of all operations in the WAL while also maintaining the integrity and consistency of the collection's data structures.
    let mut serialized: Vec<(Uuid, Vec<u8>)> = Vec::with_capacity(entries.len());
    for entry in &entries {
        let bytes = bincode::serialize(entry)?;
        serialized.push((entry.id, bytes));
    }
    // Calculate the total size required to write all new entries and grow the memory-mapped file if necessary. We sum the lengths of all serialized entries and add that to the current offset to determine the required size of the memory-mapped file. If the required size exceeds the current size of the memory-mapped file, we call the grow_mmap_if_needed function to resize the underlying file and create a new memory map with the updated size. This ensures that we have enough space to write all new entries without running into out-of-bounds errors.
    let current_offset = storage.index.values()
        .map(|idx| idx.offset + idx.length as u64)
        .max()
        .unwrap_or(0);
    
    //  Calculate the total size required to write all new entries and grow the memory-mapped file if necessary. We sum the lengths of all serialized entries and add that to the current offset to determine the required size of the memory-mapped file. If the required size exceeds the current size of the memory-mapped file, we call the grow_mmap_if_needed function to resize the underlying file and create a new memory map with the updated size. This ensures that we have enough space to write all new entries without running into out-of-bounds errors.
    let total_bytes: u64 = serialized.iter().map(|(_, b)| b.len() as u64).sum();
    let required_size = current_offset + total_bytes;
    
    // Grow the memory-mapped file if needed to accommodate all new entries. This involves unmapping the current memory map, resizing the underlying file, and creating a new memory map with the updated size. By ensuring that the memory-mapped file is large enough to hold all new entries, we can safely write the serialized data without risking out-of-bounds errors or data corruption.
    grow_mmap_if_needed(&mut storage.mmap, &storage.data_file, required_size)?;
    
    let mut offset = current_offset;
    let mmap = storage.mmap.as_mut().unwrap();

    // Write each serialized entry to the memory-mapped file at the calculated offset. For each entry, we copy the bytes to the appropriate location in the memory map, create an index entry that records the offset and length of the entry, and insert this entry into the main index of the collection. We also keep track of the IDs of the inserted entries in a vector, which will be returned at the end of the function.
    for (id, bytes) in &serialized {
        mmap[offset as usize..(offset as usize + bytes.len())]
            .copy_from_slice(bytes);
        
        let index_entry = EntryPointer {
            offset,
            length: bytes.len() as u32,
        };
        storage.index.insert(*id, index_entry);
        ids.push(*id);
        
        offset += bytes.len() as u64;
    }
    // After writing all entries to the memory-mapped file and updating the index, we need to update the vector index and cache with the new entries. We iterate through each entry, extract the vector, and insert it into the in-memory cache and the vector index. This ensures that all new entries are included in future search operations and that their vectors are readily available for similarity calculations.
    super::persistence::save_index(storage)?;
    storage.track_operation()?;
    // Update the collection metadata with the new vector count. After inserting the new entries, we need to update the metadata to reflect the new total number of vectors in the collection. This is important for maintaining accurate metadata information, which can be used for various purposes such as validating operations, providing insights about the collection, and ensuring that the collection's state is consistent with its contents.
    for entry in entries {
        let vec_f32 = entry.get_vector();
        storage.vector_cache.insert(entry.id, vec_f32.clone());
        storage.vector_index.insert(entry.id, &vec_f32, &storage.vector_cache);
    }
    
    Ok(ids)
}

pub fn upsert(storage: &mut Collection, entry: Document) -> Result<Uuid> {
    // For an upsert operation, if the document already exists, we treat it as an update. This involves deleting the existing entry and then inserting the new entry with the updated information. By doing this, we ensure that the index and vector index are properly updated to reflect the changes in the document, and that the WAL accurately captures the update operation for durability and recovery purposes.

    let id = entry.id;
    if storage.index.contains_key(&id) {
        let vector = entry.get_vector();
        let mut wal_entry = WalEntry::Update {
            id,
            vector,
            text: entry.text.clone(),
            metadata: entry.metadata.clone(),
            seq: 0,
        }; // this means we need to log an update to the WAL instead of an insert
        storage.persistence.wal.log(&mut wal_entry)?;
        

        delete_internal(storage, &id);
        insert_internal(storage, entry)?;
        super::persistence::save_index(storage)?;
        super::persistence::save_vector_index(storage)?;
        storage.track_operation()?;
        Ok(id)
    } else {
        insert(storage, entry)
    }
}

pub fn delete(storage: &mut Collection, id: &Uuid) -> Result<bool> {
    // For a delete operation, we first check if the document exists in the collection. If it does, we log a delete entry to the WAL to ensure that the deletion is recorded for durability and recovery purposes. After logging the delete operation, we proceed to remove the entry from the index, vector index, and in-memory caches. Finally, we save the updated index and vector index to disk and track the operation for checkpointing purposes. If the document does not exist, we simply return false to indicate that no deletion occurred.
    if storage.index.contains_key(id) {
        let mut wal_entry = WalEntry::Delete { id: *id, seq: 0 };
        storage.persistence.wal.log(&mut wal_entry)?;
        
        delete_internal(storage, id);
        super::persistence::save_index(storage)?;
        super::persistence::save_vector_index(storage)?;
        storage.track_operation()?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn delete_batch(storage: &mut Collection, ids: &[Uuid]) -> Result<usize> {
    // For a batch delete operation, we first iterate through the list of IDs and log a delete entry to the WAL for each ID that exists in the collection. This ensures that all delete operations are recorded in the WAL for durability and recovery purposes. After logging the delete operations, we proceed to remove each existing entry from the index, vector index, and in-memory caches. We keep track of the number of successfully deleted entries, and if any entries were deleted, we save the updated index and vector index to disk and track the operation for checkpointing purposes. Finally, we return the count of deleted entries.
    let mut deleted_count = 0;
    
    for id in ids {
        if storage.index.contains_key(id) {
            let mut wal_entry = WalEntry::Delete { id: *id, seq: 0 };
            storage.persistence.wal.log(&mut wal_entry)?;
        }
    }
    
    for id in ids {
        if storage.index.contains_key(id) {
            delete_internal(storage, id);
            deleted_count += 1;
        }
    }
    
    if deleted_count > 0 {
        super::persistence::save_index(storage)?;
        super::persistence::save_vector_index(storage)?;
        storage.track_operation()?;
    }
    
    Ok(deleted_count)
}

pub fn update_metadata(storage: &mut Collection, id: &Uuid, metadata: Metadata) -> Result<bool> {
    // For an update metadata operation, we first check if the document exists in the collection. If it does, we log an update entry to the WAL with the new metadata to ensure that the change is recorded for durability and recovery purposes. After logging the update operation, we retrieve the existing document, update its metadata, and then perform a delete followed by an insert to ensure that the index and vector index are properly updated to reflect the changes in the document. Finally, we save the updated index and vector index to disk and track the operation for checkpointing purposes. If the document does not exist, we simply return false to indicate that no update occurred.
    if let Some(entry) = get(storage, id) {
        let vector = entry.get_vector();
        
        let mut wal_entry = WalEntry::Update {
            id: *id,
            vector,
            text: entry.text.clone(),
            metadata: metadata.clone(),
            seq: 0,
        };
        storage.persistence.wal.log(&mut wal_entry)?;
        
        let mut entry = entry;
        entry.metadata = metadata;
        delete(storage, id)?;
        insert(storage, entry)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn update_vector(storage: &mut Collection, id: &Uuid, vector: Vec<f32>) -> Result<bool> {
    // For an update vector operation, we first check if the document exists in the collection. If it does, we log an update entry to the WAL with the new vector to ensure that the change is recorded for durability and recovery purposes. After logging the update operation, we retrieve the existing document, update its vector, and then perform a delete followed by an insert to ensure that the index and vector index are properly updated to reflect the changes in the document. Finally, we save the updated index and vector index to disk and track the operation for checkpointing purposes. If the document does not exist, we simply return false to indicate that no update occurred.
    if let Some(entry) = get(storage, id) {
        let mut wal_entry = WalEntry::Update {
            id: *id,
            vector: vector.clone(),
            text: entry.text.clone(),
            metadata: entry.metadata.clone(),
            seq: 0,
        };
        storage.persistence.wal.log(&mut wal_entry)?;
        
        let mut entry = entry;
        entry.vector = QuantizedVector::from_f32(&vector);
        delete(storage, id)?;
        
        insert(storage, entry)?;
        Ok(true)
    } else {
        Ok(false)
    }
}
