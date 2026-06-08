// Index utilities for vector storage

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::error::{Result, StorageError};

//  maps UUID to location in mmap file
// This is just file storage metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryPointer {
    pub offset: u64, // byte offset in file
    pub length: u32, // size of serialized entry
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

    let mut index_file = match std::fs::File::open(&index_path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(HashMap::new()),
        Err(error) => return Err(error.into()),
    };

    use std::io::Read;
    let mut index_data = Vec::new();
    index_file.read_to_end(&mut index_data)?;
    bincode::deserialize(&index_data).map_err(|e| {
        StorageError::CorruptedIndex(format!("failed to decode {index_path}: {e}")).into()
    })
}

pub fn get_wal_path(storage_path: &str) -> String {
    format!("{}.wal.db", storage_path)
}
