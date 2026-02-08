// Index utilities for vector storage

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::Result;

// Entry pointer: maps UUID to location in mmap file
// This is NOT the VectorIndex trait (which is for search algorithms)
// This is just file storage metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryPointer {
    pub offset: u64,      // byte offset in file
    pub length: u32,      // size of serialized entry
}

impl EntryPointer {
    pub fn new(offset: u64, length: u32) -> Self {
        Self { offset, length }
    }
}

pub fn save_index(path: &str, index: &HashMap<Uuid, EntryPointer>) -> Result<()> {
    let index_path = format!("{}.index.db", path);
    let index_data = bincode::serialize(index)?;
    std::fs::write(index_path, index_data)?;
    Ok(())
}

pub fn load_index(path: &str) -> Result<HashMap<Uuid, EntryPointer>> {
    let index_path = format!("{}.index.db", path);
    
    if let Ok(mut index_file) = std::fs::File::open(&index_path) {
        use std::io::Read;
        let mut index_data = Vec::new();
        if index_file.read_to_end(&mut index_data).is_ok() {
            Ok(bincode::deserialize(&index_data).unwrap_or_else(|_| HashMap::new()))
        } else {
            Ok(HashMap::new())
        }
    } else {
        Ok(HashMap::new())
    }
}

pub fn get_wal_path(storage_path: &str) -> String {
    format!("{}.wal.db", storage_path)
}
