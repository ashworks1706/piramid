// Collection checkpoint manager. Owns WAL state and collection-level checkpoint bookkeeping.

use super::collection::Collection;
use crate::error::Result;
use crate::storage::persistence::{
    save_index as save_idx, save_metadata as save_meta, save_vector_index as save_vec_idx,
};
use crate::storage::wal::Wal;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

pub struct CheckpointManager {
    pub wal: Wal, // The write-ahead log instance for managing durability and recovery
    operation_count: usize, // Counter for the number of operations since the last checkpoint
    last_checkpoint_ts: Option<u64>, // Timestamp of the last checkpoint for recovery purposes
}

impl CheckpointManager {
    pub fn new(wal: Wal) -> Self {
        Self {
            wal,
            operation_count: 0,
            last_checkpoint_ts: None,
        }
    }

    pub fn should_checkpoint(&mut self, cfg: &crate::config::WalConfig) -> bool {
        if !cfg.enabled {
            // If WAL is not enabled, we don't need to checkpoint, so we can return false immediately.
            return false;
        }
        self.operation_count += 1;
        self.operation_count >= cfg.checkpoint_frequency
    }

    pub fn reset_counter(&mut self) {
        self.operation_count = 0;
    }

    pub fn record_checkpoint(&mut self, ts: u64) {
        self.last_checkpoint_ts = Some(ts);
    }

    pub fn last_checkpoint(&self) -> Option<u64> {
        self.last_checkpoint_ts
    }
}

pub fn save_index(storage: &Collection) -> Result<()> {
    save_idx(&storage.path, &storage.index)
}

pub fn save_vector_index(storage: &Collection) -> Result<()> {
    save_vec_idx(&storage.path, storage.vector_index.as_ref()) // We pass a reference to the vector index to the save function, which will handle serializing and writing it to disk.
}

pub fn save_metadata(storage: &Collection) -> Result<()> {
    save_meta(&storage.path, &storage.metadata) // need to save the metadata of the collection during checkpoints. contains their IDs and any associated metadata fields
}

fn wal_meta_path(path: &str) -> PathBuf {
    PathBuf::from(format!("{}.wal.meta", path)) // The path for the WAL metadata file is constructed by appending ".wal.meta" to the base path of the collection.
}

#[derive(Serialize, Deserialize, Default)]
struct WalMeta {
    last_checkpoint_seq: u64,
}

pub fn load_wal_meta(path: &str) -> Result<u64> {
    let meta_path = wal_meta_path(path);
    let data = match fs::read(&meta_path) {
        Ok(data) => data,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error.into()),
    };
    let meta: WalMeta = serde_json::from_slice(&data)?;
    Ok(meta.last_checkpoint_seq)
}

fn save_wal_meta(path: &str, last_checkpoint_seq: u64) -> Result<()> {
    let meta_path = wal_meta_path(path);
    let tmp_path = meta_path.with_extension("tmp");
    let meta = WalMeta {
        last_checkpoint_seq,
    };
    fs::write(&tmp_path, serde_json::to_vec(&meta)?)?;
    fs::rename(&tmp_path, &meta_path)?;
    let file = fs::File::open(&meta_path)?;
    file.sync_all()?;
    Ok(())
}

pub fn checkpoint(storage: &mut Collection) -> Result<()> {
    // This timestamp can be used for recovery purposes to determine the point in time at which the checkpoint was taken,
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // serializing the in-memory data structures and writing them to their respective files on disk. ensure that we have a consistent snapshot of the collection's state that can be used for recovery if needed.
    save_index(storage)?;
    save_vector_index(storage)?;
    save_metadata(storage)?;

    // If WAL is enabled in the configuration, checkpoint the WAL to ensure flushing any buffered entries and rotating the log file if it exceeds the configured size or if a checkpoint is triggered based on the operation count.
    if storage.config.wal.enabled {
        storage.checkpoint.wal.checkpoint(timestamp)?;
        storage.checkpoint.record_checkpoint(timestamp);
        let last_seq = storage.checkpoint.wal.next_seq.saturating_sub(1);
        save_wal_meta(&storage.path, last_seq)?;
        storage.checkpoint.wal.rotate()?;
    }

    Ok(())
}

pub fn flush(storage: &mut Collection) -> Result<()> {
    // If WAL is enabled, we need to flush any pending entries to disk to ensure durability.
    storage.checkpoint.wal.flush()?;
    Ok(())
}
