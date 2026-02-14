// Metadata persistence utilities

use std::fs;
use std::path::Path;
use crate::error::Result;
use crate::storage::CollectionMetadata;
use crate::storage::metadata::SCHEMA_VERSION;
use crate::error::PiramidError;

// Get the metadata file path for a collection
pub fn get_metadata_path(collection_path: &str) -> String {
    format!("{}.metadata.db", collection_path)
}

// Save collection metadata to disk
pub fn save_metadata(collection_path: &str, metadata: &CollectionMetadata) -> Result<()> {
    let bytes = bincode::serialize(metadata)?;
    let metadata_path = get_metadata_path(collection_path);
    fs::write(metadata_path, bytes)?;
    Ok(())
}

// Load collection metadata from disk
pub fn load_metadata(collection_path: &str) -> Result<Option<CollectionMetadata>> {
    let metadata_path = get_metadata_path(collection_path);
    
    if !Path::new(&metadata_path).exists() {
        return Ok(None);
    }
    
    let bytes = fs::read(metadata_path)?;
    let metadata: CollectionMetadata = bincode::deserialize(&bytes).map_err(|e| {
        PiramidError::Storage(crate::error::storage::StorageError::CorruptedData(format!(
            "Failed to read metadata: {e}"
        )))
    })?;
    if metadata.schema_version != SCHEMA_VERSION {
        return Err(PiramidError::Storage(
            crate::error::storage::StorageError::CorruptedData(format!(
                "Schema version mismatch: expected {}, found {}",
                SCHEMA_VERSION, metadata.schema_version
            ))
        ).into());
    }
    Ok(Some(metadata))
}
