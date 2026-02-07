// Index utilities for vector storage

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::Result;

// Index entry: maps UUID to location in mmap file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorIndex {
    pub offset: u64,      // byte offset in file
    pub length: u32,      // size of serialized entry
}

impl VectorIndex {
    pub fn new(offset: u64, length: u32) -> Self {
        Self { offset, length }
    }
}

pub fn save_index(path: &str, index: &HashMap<Uuid, VectorIndex>) -> Result<()> {
    let index_path = if path.ends_with(".db") {
        format!("{}.index.db", &path[..path.len()-3])
    } else {
        format!("{}.index", path)
    };
    let index_data = bincode::serialize(index)?;
    std::fs::write(index_path, index_data)?;
    Ok(())
}

pub fn load_index(path: &str) -> Result<HashMap<Uuid, VectorIndex>> {
    let index_path = if path.ends_with(".db") {
        format!("{}.index.db", &path[..path.len()-3])
    } else {
        format!("{}.index", path)
    };
    
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
    if storage_path.ends_with(".db") {
        format!("{}.wal", &storage_path[..storage_path.len()-3])
    } else {
        format!("{}.wal", storage_path)
    }
}
