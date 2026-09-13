//! Faulting pages in before first use, whichever way the bytes are read.

use memmap2::MmapMut;
use piramid_core::error::Result;
use std::fs;
use std::io::{BufReader, Read};

/// Reads a file front to back so its pages are resident before first use.
pub fn warm_file(path: &str) -> Result<()> {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    let mut reader = BufReader::new(file);
    let mut buf = vec![0u8; 4 * 1024 * 1024];
    loop {
        let read = reader.read(&mut buf)?;
        if read == 0 {
            break;
        }
        std::hint::black_box(&buf[..read]);
    }
    Ok(())
}

/// Touch each page of the mmap to fault it into memory.
pub(crate) fn warm_mmap(mmap: &MmapMut) {
    let len = mmap.len();
    if len == 0 {
        return;
    }
    const PAGE: usize = 4096;
    let mut offset: usize = 0;
    while offset < len {
        let byte = mmap[offset];
        std::hint::black_box(byte);
        offset = offset.saturating_add(PAGE);
    }
    // The stride can skip the tail page.
    let last = mmap[len - 1];
    std::hint::black_box(last);
}
