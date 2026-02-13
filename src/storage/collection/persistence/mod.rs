use crate::error::Result;
use crate::storage::persistence::{save_index as save_idx, save_vector_index as save_vec_idx, save_metadata as save_meta};
use crate::storage::wal::Wal;
use super::storage::Collection;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

pub struct PersistenceService {
    pub wal: Wal,
    operation_count: usize,
    last_checkpoint_ts: Option<u64>,
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
        if !cfg.enabled {
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
    save_vec_idx(&storage.path, storage.vector_index.as_ref())
}

pub fn save_metadata(storage: &Collection) -> Result<()> {
    save_meta(&storage.path, &storage.metadata)
}

fn wal_meta_path(path: &str) -> PathBuf {
    PathBuf::from(format!("{}.wal.meta", path))
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
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    save_index(storage)?;
    save_vector_index(storage)?;
    save_metadata(storage)?;
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
    storage.persistence.wal.flush()?;
    Ok(())
}
