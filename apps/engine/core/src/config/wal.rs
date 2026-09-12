//! Write-ahead log configuration.

use serde::{Deserialize, Serialize};

/// Write-ahead logging and checkpoint cadence for a collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct WalConfig {
    /// Log every write before applying it. Off skips the log and replay on open.
    pub enabled: bool,

    /// Checkpoint after this many operations.
    pub checkpoint_frequency: usize,

    /// Also checkpoint after this many seconds, if set.
    #[serde(default)]
    pub checkpoint_interval_secs: Option<u64>,

    /// Rotate once the log passes this many bytes.
    pub max_log_size: usize,

    /// Call fsync on every write.
    pub sync_on_write: bool,
}

impl Default for WalConfig {
    fn default() -> Self {
        WalConfig {
            enabled: true,
            checkpoint_frequency: 1000,
            checkpoint_interval_secs: None,
            max_log_size: 100 * 1024 * 1024,
            sync_on_write: false,
        }
    }
}
