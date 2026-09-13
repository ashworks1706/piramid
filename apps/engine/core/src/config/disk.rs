//! Disk-pressure policy.

use serde::{Deserialize, Serialize};

/// What to do as the data directory's filesystem fills up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct DiskConfig {
    /// Refuse writes below this much free space. None never checks.
    pub min_free_bytes: Option<u64>,

    /// Drop to read-only at the threshold. Off fails each write below it.
    pub readonly_on_low_space: bool,
}

impl Default for DiskConfig {
    fn default() -> Self {
        DiskConfig {
            min_free_bytes: None,
            readonly_on_low_space: true,
        }
    }
}
