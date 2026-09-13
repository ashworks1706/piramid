//! Memory-mapped file helpers backing the collection record store.
use memmap2::{MmapMut, MmapOptions};
use std::fs::File;

use piramid_core::error::Result;

pub fn ensure_file_size(file: &File, min_size: u64) -> Result<()> {
    let current_size = file.metadata()?.len();
    if current_size < min_size {
        file.set_len(min_size)?;
    }
    Ok(())
}

/// Creates a mutable memory map over a file. Call [ensure_file_size] first.
#[allow(unsafe_code)]
pub fn create_mmap(file: &File) -> Result<MmapMut> {
    // SAFETY: the caller holds exclusive ownership of the file for the life of the mapping.
    unsafe { Ok(MmapOptions::new().map_mut(file)?) }
}

pub fn grow_mmap_if_needed(
    mmap: &mut Option<MmapMut>,
    file: &File,
    required_size: u64,
) -> Result<()> {
    let current_size = mapped_or_file_len(mmap.as_deref(), file)?;
    if required_size > current_size {
        let new_size = required_size.saturating_mul(2);
        if mmap.is_some() {
            drop(mmap.take());
            file.set_len(new_size)?;
            *mmap = Some(create_mmap(file)?);
        } else {
            file.set_len(new_size)?;
        }
    }
    Ok(())
}

/// Bytes currently addressable: the length of the mapping, or of the file when there is none.
pub fn mapped_or_file_len(mmap: Option<&[u8]>, file: &File) -> Result<u64> {
    match mmap {
        Some(mmap) => Ok(mmap.len() as u64),
        None => Ok(file.metadata()?.len()),
    }
}
