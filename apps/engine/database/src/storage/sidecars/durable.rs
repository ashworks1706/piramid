//! File replacement that survives a crash: every rename is followed by a sync of its directory.

use std::fs;
use std::path::Path;

use piramid_core::error::Result;

/// Fsync the directory holding path.
pub(crate) fn sync_parent_dir(path: &str) -> Result<()> {
    let parent = match Path::new(path).parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    fs::File::open(parent)?.sync_all()?;
    Ok(())
}

/// Rename from over to and sync the directory, so the rename is durable once this returns.
pub(crate) fn replace_file(from: &str, to: &str) -> Result<()> {
    fs::rename(from, to)?;
    sync_parent_dir(to)
}

/// Path of the temporary file [write_atomic] writes before renaming it over path.
pub(crate) fn tmp_path(path: &str) -> String {
    format!("{path}.tmp")
}

/// Write bytes to path through a synced temporary file and a durable rename, so path holds either
/// its old contents or bytes.
pub(crate) fn write_atomic(path: &str, bytes: &[u8]) -> Result<()> {
    let tmp_path = tmp_path(path);
    let file = fs::File::create(&tmp_path)?;
    std::io::Write::write_all(&mut &file, bytes)?;
    file.sync_all()?;
    drop(file);
    replace_file(&tmp_path, path)
}

/// Remove path, treating a missing file as removed.
pub(crate) fn remove_if_present(path: &str) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}
