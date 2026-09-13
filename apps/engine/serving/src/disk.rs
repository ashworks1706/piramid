//! Filesystem capacity probing, the one unsafe site in piramid-serving.

use piramid_core::error::{Result, ServerError};

/// Total and available bytes on the filesystem backing path, or (None, None) off Unix.
#[cfg_attr(not(target_family = "unix"), allow(unused_variables))]
pub fn stats(path: &str) -> Result<(Option<u64>, Option<u64>)> {
    #[cfg(target_family = "unix")]
    {
        use std::ffi::CString;

        let c_path = CString::new(path)
            .map_err(|_| ServerError::Internal("data_dir contains an interior NUL byte".into()))?;

        // SAFETY: statvfs(3) fills every field before it is read, and c_path outlives the call.
        #[allow(unsafe_code)]
        let (rc, stat) = unsafe {
            let mut stat: libc::statvfs = std::mem::zeroed();
            let rc = libc::statvfs(c_path.as_ptr(), &mut stat);
            (rc, stat)
        };

        if rc == 0 {
            // fsblkcnt_t and f_frsize are u64 on Linux and narrower on other targets.
            #[allow(clippy::unnecessary_cast)]
            let (total, available) = {
                let frsize = stat.f_frsize as u64;
                (
                    (stat.f_blocks as u64).saturating_mul(frsize),
                    (stat.f_bavail as u64).saturating_mul(frsize),
                )
            };
            return Ok((Some(total), Some(available)));
        }
        Err(std::io::Error::last_os_error().into())
    }
    #[cfg(not(target_family = "unix"))]
    {
        Ok((None, None))
    }
}

/// Available bytes on the filesystem backing path.
pub fn free_bytes(path: &str) -> Result<Option<u64>> {
    stats(path).map(|(_, available)| available)
}
