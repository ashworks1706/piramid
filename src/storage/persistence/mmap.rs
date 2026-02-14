// Memory-mapped file utilities
// This module provides utilities for working with memory-mapped files, including ensuring file size, creating memory maps, and growing memory maps as needed. These utilities are used in the collection storage implementation to manage the memory-mapped file that stores the collection's data.
use memmap2::{MmapMut, MmapOptions};
use std::fs::File;

use crate::error::Result;

pub fn ensure_file_size(file: &File, min_size: u64) -> Result<()> {
    let current_size = file.metadata()?.len();
    if current_size < min_size {
        file.set_len(min_size)?;
    }
    Ok(())
}

pub fn create_mmap(file: &File) -> Result<MmapMut> {
    unsafe { Ok(MmapOptions::new().map_mut(file)?) }
}

pub fn grow_mmap_if_needed(
    mmap: &mut Option<MmapMut>,
    file: &File,
    required_size: u64,
) -> Result<()> {

    let current_size = mmap.as_ref().unwrap().len() as u64;
    if required_size > current_size {
        drop(mmap.take());
        file.set_len(required_size * 2)?;
        *mmap = Some(create_mmap(file)?);
    } // If the required size is within the current size, we can simply continue using the existing memory map without any changes.
    Ok(())
}
