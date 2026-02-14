// This module defines the persistence service for the collection, which is responsible for managing the write-ahead log (WAL) and performing checkpoints to save the state of the collection to disk. It provides functions to save the index, vector index, and metadata of the collection, as well as to load and save WAL metadata. The checkpoint function saves the current state of the collection and rotates the WAL if necessary, while the flush function ensures that all pending WAL entries are flushed to disk. The persistence service also includes logic to determine when a checkpoint should be performed based on the configured checkpoint frequency and to record the timestamp of the last checkpoint for recovery purposes.

use crate::error::Result;
use crate::storage::persistence::{save_index as save_idx, save_vector_index as save_vec_idx, save_metadata as save_meta};
use crate::storage::wal::Wal;
use super::storage::Collection;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

pub struct PersistenceService {
    pub wal: Wal, // The write-ahead log instance for managing durability and recovery
    operation_count: usize, // Counter for the number of operations since the last checkpoint
    last_checkpoint_ts: Option<u64>, // Timestamp of the last checkpoint for recovery purposes
}


impl PersistenceService {
    pub fn new(wal: Wal) -> Self {
        Self {
            wal,
            operation_count: 0,
            last_checkpoint_ts: None,
        }
    }

    pub fn should_checkpoint(&mut self, cfg: &crate::config::WalConfig) -> bool {
        if !cfg.enabled { // If WAL is not enabled, we don't need to checkpoint, so we can return false immediately. Checkpointing is only relevant when WAL is enabled, as it allows us to save the state of the collection and rotate the WAL to prevent it from growing indefinitely. If WAL is disabled, we can skip all checkpointing logic and just return false.
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
    save_vec_idx(&storage.path, storage.vector_index.as_ref()) // We pass a reference to the vector index to the save function, which will handle serializing and writing it to disk. The vector index is a critical component of the collection that allows for efficient similarity search, so it's important to ensure that it is saved correctly during checkpoints. By saving the vector index along with the main index and metadata, we can ensure that we have a consistent state of the collection that can be recovered in case of a crash or unexpected shutdown.
}

pub fn save_metadata(storage: &Collection) -> Result<()> {
    save_meta(&storage.path, &storage.metadata) // Similar to saving the index and vector index, we also need to save the metadata of the collection during checkpoints. The metadata contains important information about the documents in the collection, such as their IDs and any associated metadata fields. By saving the metadata along with the index and vector index, we can ensure that we have a complete snapshot of the collection's state that can be used for recovery if needed.
}

fn wal_meta_path(path: &str) -> PathBuf {
    PathBuf::from(format!("{}.wal.meta", path)) // The path for the WAL metadata file is constructed by appending ".wal.meta" to the base path of the collection. This file will be used to store information about the last checkpoint sequence number, which is important for determining where to start replaying the WAL during recovery. By keeping this metadata in a separate file, we can easily manage and update it without affecting the main collection data files.
}

#[derive(Serialize, Deserialize, Default)]
struct WalMeta {
    last_checkpoint_seq: u64,
}

pub fn load_wal_meta(path: &str) -> Result<u64> {
    let meta_path = wal_meta_path(path);
    if let Ok(data) = fs::read(&meta_path) {
        let meta: WalMeta = serde_json::from_slice(&data)?;
        Ok(meta.last_checkpoint_seq)
    } else {
        Ok(0)
    }
}

fn save_wal_meta(path: &str, last_checkpoint_seq: u64) -> Result<()> {
    let meta_path = wal_meta_path(path);
    let tmp_path = meta_path.with_extension("tmp");
    let meta = WalMeta { last_checkpoint_seq };
    fs::write(&tmp_path, serde_json::to_vec(&meta)?)?;
    fs::rename(&tmp_path, &meta_path)?;
    if let Ok(file) = fs::File::open(&meta_path) {
        let _ = file.sync_all();
    }
    Ok(())
}


pub fn checkpoint(storage: &mut Collection) -> Result<()> {
    // 1. Get the current timestamp to record when the checkpoint is being performed. This timestamp can be used for recovery purposes to determine the point in time at which the checkpoint was taken, which can help in replaying the WAL entries correctly during recovery.
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // 2. Save the current state of the index, vector index, and metadata to disk. This involves serializing the in-memory data structures and writing them to their respective files on disk. By saving these components, we ensure that we have a consistent snapshot of the collection's state that can be used for recovery if needed.
    save_index(storage)?;
    save_vector_index(storage)?;
    save_metadata(storage)?;

    // 3. If WAL is enabled in the configuration, we need to checkpoint the WAL to ensure that all pending entries are flushed to disk and that the WAL is rotated if necessary. This involves calling the checkpoint method on the WAL instance, which will handle flushing any buffered entries and rotating the log file if it exceeds the configured size or if a checkpoint is triggered based on the operation count.
    if storage.config.wal.enabled {
        storage.persistence.wal.checkpoint(timestamp)?;
        storage.persistence.record_checkpoint(timestamp);
        let last_seq = storage.persistence.wal.next_seq.saturating_sub(1);
        save_wal_meta(&storage.path, last_seq)?;
        storage.persistence.wal.rotate()?;
    }

    Ok(())
}

pub fn flush(storage: &mut Collection) -> Result<()> {
    // If WAL is enabled, we need to flush any pending entries to disk to ensure durability. This involves calling the flush method on the WAL instance, which will write any buffered entries to the log file and ensure that they are persisted on disk. Flushing is important to guarantee that all operations are safely stored in the WAL before we perform a checkpoint or before shutting down the collection, as it allows us to recover from any crashes or unexpected shutdowns without losing data.
    storage.persistence.wal.flush()?;
    Ok(())
}
