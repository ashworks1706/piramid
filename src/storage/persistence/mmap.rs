// Memory-mapped file utilities
// used in the collection storage to manage memory-mapped file that stores the collection's data.
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

// Create a mutable memory map for the given file. The file must already be of sufficient size to accommodate the memory map. 
pub fn create_mmap(file: &File) -> Result<MmapMut> {
    unsafe { Ok(MmapOptions::new().map_mut(file)?) }
}

/// Touch each page of the mmap to fault it into memory.
pub fn warm_mmap(mmap: &MmapMut) {
    let len = mmap.len();
    if len == 0 {
        return;
    }
    // Step by page-sized chunks to avoid touching every byte.
    const PAGE: usize = 4096;
    let mut offset: usize = 0;
    while offset < len {
        // SAFETY: offset is within bounds and we only read.
        let byte = mmap[offset];
        std::hint::black_box(byte);
        offset = offset.saturating_add(PAGE);
    }
    // Ensure we touched the tail.
    let last = mmap[len - 1];
    std::hint::black_box(last);
}

pub fn grow_mmap_if_needed(
    mmap: &mut Option<MmapMut>,
    file: &File,
    required_size: u64,
) -> Result<()> {
    let current_size = mmap
        .as_ref()
        .map(|mmap| mmap.len() as u64)
        .unwrap_or_else(|| file.metadata().map(|meta| meta.len()).unwrap_or(0));
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
    // If the required size is within the current size, we can simply continue using the existing 
    // memory map without any changes.
    Ok(())
}
