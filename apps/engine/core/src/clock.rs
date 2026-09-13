//! Wall-clock timestamps for records that persist.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{PiramidError, Result};

/// Seconds since the Unix epoch. A clock reading before 1970 is an error.
pub fn unix_secs() -> Result<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .map_err(|error| {
            PiramidError::other(format!(
                "the system clock reads {:?} before 1970",
                error.duration()
            ))
        })
}
