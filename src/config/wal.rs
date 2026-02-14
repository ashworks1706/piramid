// Write-Ahead Log (WAL) configuration
// This configuration struct defines the parameters for the write-ahead log, which is used to ensure durability and recoverability of the collection in case of crashes or unexpected shutdowns. The WAL allows us to log changes to the collection before they are applied, so that we can replay those changes during recovery to bring the collection back to a consistent state.
use serde::{Deserialize, Serialize};

// WAL configuration
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WalConfig {
    // Enable WAL
    pub enabled: bool,
    
    // Checkpoint frequency (flush every N operations)
    pub checkpoint_frequency: usize,

    // Optional time-based checkpoint interval (seconds)
    #[serde(default)]
    pub checkpoint_interval_secs: Option<u64>,
    
    // Maximum log file size in bytes before rotation
    pub max_log_size: usize,
    
    // Sync to disk after every write (slower but safer)
    pub sync_on_write: bool,
}

impl Default for WalConfig {
    fn default() -> Self {
        WalConfig {
            enabled: true,
            checkpoint_frequency: 1000,
            checkpoint_interval_secs: None,
            max_log_size: 100 * 1024 * 1024,  // 100MB
            sync_on_write: false,
        }
    }
}

impl WalConfig {
    // Disable WAL (not recommended for production)
    pub fn disabled() -> Self {
        WalConfig {
            enabled: false,
            checkpoint_frequency: 0,
            max_log_size: 0,
            sync_on_write: false,
            checkpoint_interval_secs: None,
        }
    }
    
    // High durability mode (sync on every write)
    pub fn high_durability() -> Self {
        WalConfig {
            enabled: true,
            checkpoint_frequency: 100,
            max_log_size: 50 * 1024 * 1024,  // 50MB
            sync_on_write: true,
            checkpoint_interval_secs: Some(1),
        }
    }
    
    // Fast mode (larger checkpoint intervals)
    pub fn fast() -> Self {
        WalConfig {
            enabled: true,
            checkpoint_frequency: 10000,
            max_log_size: 500 * 1024 * 1024,  // 500MB
            sync_on_write: false,
            checkpoint_interval_secs: None,
        }
    }
}
