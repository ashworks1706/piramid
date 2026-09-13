//! One owner for every sidecar path and format beside the record file of a collection.

use std::collections::HashMap;
use std::fs;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::durable::write_atomic;
use super::offsets::EntryPointer;
use crate::storage::codec;
use crate::storage::manifest::CollectionMetadata;
use piramid_core::error::{Result, StorageError};

/// The sidecar domain entry for one collection.
///
/// Every file beside the base path gets its path and its serialization from here: offsets,
/// manifest, WAL, WAL meta and the files of an interrupted compaction.
#[derive(Clone, Copy)]
pub struct SidecarManager<'a> {
    base: &'a str,
}

/// Checkpoint bookkeeping persisted beside the WAL.
#[derive(Serialize, Deserialize, Default)]
struct WalMeta {
    last_checkpoint_seq: u64,
}

impl<'a> SidecarManager<'a> {
    /// The sidecars beside the record file at the base path.
    pub fn at(base: &'a str) -> Self {
        Self { base }
    }

    /// Every suffix this type appends to a base path, including the index sidecar Piramid 0.2
    /// wrote, which this build never reads.
    pub const SUFFIXES: [&'static str; 8] = [
        ".wal.db",
        ".wal.meta",
        ".offsets.db",
        ".manifest.db",
        ".vecindex.db",
        ".compact",
        ".compact.offsets",
        ".compact.commit",
    ];

    /// Every sidecar path beside this base, existing or not.
    pub fn all_paths(&self) -> Vec<String> {
        Self::SUFFIXES
            .iter()
            .map(|suffix| format!("{}{suffix}", self.base))
            .collect()
    }

    /// Path of the write-ahead log.
    pub fn wal_path(&self) -> String {
        format!("{}.wal.db", self.base)
    }

    /// Path of the WAL checkpoint bookkeeping file.
    pub fn wal_meta_path(&self) -> String {
        format!("{}.wal.meta", self.base)
    }

    /// Path of the offset-index sidecar: uuid to a byte range in the record file.
    pub fn offsets_path(&self) -> String {
        format!("{}.offsets.db", self.base)
    }

    /// Path of the manifest sidecar: schema version, name, dimensions, counts.
    pub fn manifest_path(&self) -> String {
        format!("{}.manifest.db", self.base)
    }

    /// Path of the record file a compaction rewrites live documents into.
    pub fn compact_path(&self) -> String {
        format!("{}.compact", self.base)
    }

    /// Path of the offsets a compaction writes for the records in [SidecarManager::compact_path].
    pub fn compact_offsets_path(&self) -> String {
        format!("{}.compact.offsets", self.base)
    }

    /// Path of the marker whose presence commits a compaction: open moves the compacted files into
    /// place when it exists and discards them when it does not.
    pub fn compact_commit_path(&self) -> String {
        format!("{}.compact.commit", self.base)
    }

    /// Persist the offset index.
    pub fn save_offsets(&self, offsets: &HashMap<Uuid, EntryPointer>) -> Result<()> {
        Self::write_bincode(&self.offsets_path(), offsets)
    }

    /// Persist the offsets of a compacted record file to [SidecarManager::compact_offsets_path].
    pub fn save_compact_offsets(&self, offsets: &HashMap<Uuid, EntryPointer>) -> Result<()> {
        Self::write_bincode(&self.compact_offsets_path(), offsets)
    }

    /// Load the offset index. A missing sidecar is an empty collection.
    pub fn load_offsets(&self) -> Result<HashMap<Uuid, EntryPointer>> {
        let path = self.offsets_path();
        let Some(data) = Self::read_optional(&path)? else {
            return Ok(HashMap::new());
        };
        codec::decode(&data).map_err(|e| {
            StorageError::CorruptedSidecar(format!("failed to decode {path}: {e}")).into()
        })
    }

    /// Persist the collection manifest.
    pub fn save_manifest(&self, metadata: &CollectionMetadata) -> Result<()> {
        Self::write_bincode(&self.manifest_path(), metadata)
    }

    /// Load the manifest, refusing one written under a different schema version as
    /// [CollectionMetadata::decode] does.
    pub fn load_manifest(&self) -> Result<Option<CollectionMetadata>> {
        let path = self.manifest_path();
        let Some(bytes) = Self::read_optional(&path)? else {
            return Ok(None);
        };
        CollectionMetadata::decode(&bytes).map(Some)
    }

    /// Record the last checkpointed WAL sequence, atomically via a temp file.
    pub fn save_wal_meta(&self, last_checkpoint_seq: u64) -> Result<()> {
        let meta = WalMeta {
            last_checkpoint_seq,
        };
        write_atomic(&self.wal_meta_path(), &serde_json::to_vec(&meta)?)
    }

    /// Last checkpointed WAL sequence, 0 when no checkpoint has happened.
    pub fn load_wal_meta(&self) -> Result<u64> {
        let Some(data) = Self::read_optional(&self.wal_meta_path())? else {
            return Ok(0);
        };
        let meta: WalMeta = serde_json::from_slice(&data)?;
        Ok(meta.last_checkpoint_seq)
    }

    /// Serializes a value with bincode and writes it to a path atomically.
    fn write_bincode<T: Serialize>(path: &str, value: &T) -> Result<()> {
        write_atomic(path, &codec::encode(value)?)
    }

    /// Reads a path, returning None when the file is missing.
    fn read_optional(path: &str) -> Result<Option<Vec<u8>>> {
        match fs::read(path) {
            Ok(data) => Ok(Some(data)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }
}
