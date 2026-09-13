//! Checkpoint bookkeeping: when to flush sidecars, and clearing the WAL once they are durable.

use super::Collection;
use crate::storage::wal::Wal;
use crate::storage::SidecarManager;
use piramid_core::error::Result;

/// A collection's write-ahead log and the counters that decide when it checkpoints.
pub struct CheckpointManager {
    /// The log writes are recorded in.
    pub wal: Wal,
    operation_count: usize,
    last_checkpoint_ts: Option<u64>,
    /// When the current checkpoint interval began: the open, or the last checkpoint.
    interval_start_ts: u64,
}

impl CheckpointManager {
    /// A manager over wal with no operations counted and no checkpoint recorded. Errors when the
    /// clock reads before 1970.
    pub fn new(wal: Wal) -> Result<Self> {
        Ok(Self {
            wal,
            operation_count: 0,
            last_checkpoint_ts: None,
            interval_start_ts: piramid_core::clock::unix_secs()?,
        })
    }

    /// Whether this operation should be followed by a checkpoint.
    ///
    /// Three independent triggers: operation count, time since the interval began, and log size.
    /// Errors when the log size cannot be read.
    pub fn should_checkpoint(
        &mut self,
        cfg: &piramid_core::config::WalConfig,
        now: u64,
    ) -> Result<bool> {
        if !cfg.enabled {
            return Ok(false);
        }
        self.operation_count += 1;

        if self.operation_count >= cfg.checkpoint_frequency {
            return Ok(true);
        }
        if let Some(interval) = cfg.checkpoint_interval_secs {
            if now.saturating_sub(self.interval_start_ts) >= interval {
                return Ok(true);
            }
        }
        Ok(self
            .wal
            .size_bytes()?
            .is_some_and(|bytes| bytes >= cfg.max_log_size as u64))
    }

    /// Zero the operation count.
    pub fn reset_counter(&mut self) {
        self.operation_count = 0;
    }

    /// Record ts, in Unix seconds, as the time of the last checkpoint and the start of the next
    /// checkpoint interval.
    pub fn record_checkpoint(&mut self, ts: u64) {
        self.last_checkpoint_ts = Some(ts);
        self.interval_start_ts = ts;
    }

    /// Unix seconds of the last checkpoint since open. None before the first.
    pub fn last_checkpoint(&self) -> Option<u64> {
        self.last_checkpoint_ts
    }
}

pub fn checkpoint(collection: &mut Collection) -> Result<()> {
    collection.ensure_writable()?;
    let timestamp = piramid_core::clock::unix_secs()?;

    // The records, the manifest and the offsets are durable before the WAL is cleared below.
    collection.record_store.sync()?;
    let sidecars = SidecarManager::at(&collection.path);
    sidecars.save_manifest(&collection.manifest)?;
    sidecars.save_offsets(&collection.offsets)?;

    if collection.config.wal.enabled {
        collection.checkpoint.wal.checkpoint(timestamp)?;
        collection.checkpoint.record_checkpoint(timestamp);
        let last_seq = collection.checkpoint.wal.next_seq.saturating_sub(1);
        sidecars.save_wal_meta(last_seq)?;
        collection.checkpoint.wal.rotate()?;
    }

    Ok(())
}

pub fn flush(collection: &mut Collection) -> Result<()> {
    collection.checkpoint.wal.flush()?;
    Ok(())
}
