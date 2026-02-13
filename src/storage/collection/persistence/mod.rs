use crate::error::Result;
use crate::storage::persistence::{save_index as save_idx, save_vector_index as save_vec_idx, save_metadata as save_meta};
use crate::storage::wal::Wal;
use super::storage::Collection;

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

pub fn checkpoint(storage: &mut Collection) -> Result<()> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    storage.persistence.wal.checkpoint(timestamp)?;
    storage.persistence.record_checkpoint(timestamp);
    save_index(storage)?;
    save_vector_index(storage)?;
    save_metadata(storage)?;
    storage.persistence.wal.truncate()?;

    Ok(())
}

pub fn flush(storage: &mut Collection) -> Result<()> {
    storage.persistence.wal.flush()?;
    Ok(())
}
