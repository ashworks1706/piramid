//! The collection manifest: name, dimensionality, counts, timestamps.

use piramid_core::error::{Result, StorageError};
use serde::{Deserialize, Serialize};

/// The manifest persisted beside a collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionMetadata {
    /// Manifest format version the file was written with.
    pub schema_version: u32,
    /// Collection name, taken from the data file's stem.
    pub name: String,
    /// Creation time in Unix seconds.
    pub created_at: u64,
    /// Last change in Unix seconds.
    pub updated_at: u64,
    /// Vector width. None until the first vector is stored.
    pub dimensions: Option<usize>,
    /// Live documents at the last update.
    pub vector_count: usize,
}

/// Manifest format version this build reads and writes.
pub const SCHEMA_VERSION: u32 = 1;

impl CollectionMetadata {
    /// A manifest for an empty collection, created now.
    pub fn new(name: String) -> Self {
        let now = piramid_core::clock::unix_secs();

        Self {
            schema_version: SCHEMA_VERSION,
            name,
            created_at: now,
            updated_at: now,
            dimensions: None,
            vector_count: 0,
        }
    }

    /// Set updated_at to now.
    pub fn touch(&mut self) {
        self.updated_at = piramid_core::clock::unix_secs();
    }

    /// Records the vector width of the collection the first time a vector is stored.
    pub fn set_dimensions(&mut self, dimensions: usize) -> Result<()> {
        match self.dimensions {
            None => {
                self.dimensions = Some(dimensions);
                Ok(())
            }
            Some(existing) if existing == dimensions => Ok(()),
            Some(existing) => Err(StorageError::InvalidDimension {
                expected: existing,
                actual: dimensions,
            }
            .into()),
        }
    }

    /// Set the live document count and touch the manifest.
    pub fn update_vector_count(&mut self, count: usize) {
        self.vector_count = count;
        self.touch();
    }
}
