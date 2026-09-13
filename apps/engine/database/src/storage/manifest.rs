//! The collection manifest: name, metric, dimensionality, counts, timestamps.

use piramid_core::error::{Result, StorageError};
use piramid_hardware::compute::Metric;
use serde::{Deserialize, Serialize};

use crate::storage::codec;

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
    /// The metric every search of the collection scores with, fixed when the collection is created.
    pub metric: Metric,
}

/// Manifest format version this build reads and writes.
pub const SCHEMA_VERSION: u32 = 2;

/// Manifest format version written by Piramid 0.2, which has no metric.
pub const LEGACY_SCHEMA_VERSION: u32 = 1;

/// The fields every manifest version starts with.
#[derive(Deserialize)]
struct ManifestHeader {
    schema_version: u32,
    name: String,
}

impl CollectionMetadata {
    /// A manifest for an empty collection measured by metric, created now. Errors when the clock
    /// reads before 1970.
    pub fn new(name: String, metric: Metric) -> Result<Self> {
        let now = piramid_core::clock::unix_secs()?;

        Ok(Self {
            schema_version: SCHEMA_VERSION,
            name,
            created_at: now,
            updated_at: now,
            dimensions: None,
            vector_count: 0,
            metric,
        })
    }

    /// Decode a manifest from its stored bytes.
    ///
    /// Errors with [StorageError::LegacyManifest] for a schema 1 manifest, with
    /// [StorageError::UnsupportedManifest] for any other version than [SCHEMA_VERSION], and with
    /// [StorageError::CorruptedData] for bytes that do not decode.
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let header: ManifestHeader = codec::decode_prefix(bytes)
            .map_err(|e| StorageError::CorruptedData(format!("failed to read manifest: {e}")))?;
        match header.schema_version {
            SCHEMA_VERSION => codec::decode(bytes).map_err(|e| {
                StorageError::CorruptedData(format!("failed to read manifest: {e}")).into()
            }),
            LEGACY_SCHEMA_VERSION => Err(StorageError::LegacyManifest {
                collection: header.name,
            }
            .into()),
            found => Err(StorageError::UnsupportedManifest {
                collection: header.name,
                found,
            }
            .into()),
        }
    }

    /// Set updated_at to now. Errors when the clock reads before 1970.
    pub fn touch(&mut self) -> Result<()> {
        self.updated_at = piramid_core::clock::unix_secs()?;
        Ok(())
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

    /// Set the live document count and touch the manifest. Errors when the clock reads before
    /// 1970, leaving the manifest unchanged.
    pub fn update_vector_count(&mut self, count: usize) -> Result<()> {
        self.touch()?;
        self.vector_count = count;
        Ok(())
    }
}
