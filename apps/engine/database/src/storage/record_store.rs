//! The append-only data file documents are written to and read from.

use memmap2::MmapMut;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};

use crate::storage::sidecars::{
    create_mmap, ensure_file_size, grow_mmap_if_needed, warm_mmap, EntryPointer,
};
use piramid_core::config::CollectionConfig;
use piramid_core::error::{Result, StorageError};
use piramid_core::Document;

/// A collection's data file, memory-mapped when the configuration allows.
pub struct RecordStore {
    data_file: File,
    mmap: Option<MmapMut>,
    append_cursor: u64,
}

impl RecordStore {
    /// Open or create the data file at path, appending after the furthest record in index.
    pub fn open(
        path: &str,
        config: &CollectionConfig,
        index: &std::collections::HashMap<uuid::Uuid, EntryPointer>,
    ) -> Result<Self> {
        let data_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;

        let initial_size = initial_size(config);
        ensure_file_size(&data_file, initial_size)?;

        let mmap = if config.memory.use_mmap {
            Some(create_mmap(&data_file)?)
        } else {
            None
        };

        Ok(Self {
            data_file,
            mmap,
            append_cursor: next_append_offset(index),
        })
    }

    /// Write bytes at the end of the file and return where they landed.
    pub fn append(&mut self, bytes: &[u8]) -> Result<EntryPointer> {
        let offset = self.append_cursor;
        let required_size = offset + bytes.len() as u64;
        grow_mmap_if_needed(&mut self.mmap, &self.data_file, required_size)?;
        self.write_at(offset, bytes)?;
        self.append_cursor = required_size;
        Ok(EntryPointer::new(offset, bytes.len() as u32))
    }

    /// The bytes a document is stored as.
    pub fn encode_document(document: &Document) -> Result<Vec<u8>> {
        Ok(bincode::serialize(document)?)
    }

    /// Write each entry's bytes at the end of the file, in order, and return where each landed.
    pub fn append_batch(&mut self, entries: &[(uuid::Uuid, Vec<u8>)]) -> Result<Vec<EntryPointer>> {
        let total_bytes: u64 = entries.iter().map(|(_, bytes)| bytes.len() as u64).sum();
        let required_size = self.append_cursor + total_bytes;
        grow_mmap_if_needed(&mut self.mmap, &self.data_file, required_size)?;

        let mut pointers = Vec::with_capacity(entries.len());
        for (_, bytes) in entries {
            let offset = self.append_cursor;
            self.write_at(offset, bytes)?;
            self.append_cursor += bytes.len() as u64;
            pointers.push(EntryPointer::new(offset, bytes.len() as u32));
        }
        Ok(pointers)
    }

    /// Read and decode the document at pointer.
    pub fn read_document(&self, pointer: &EntryPointer) -> Result<Document> {
        let bytes = self.read_bytes(pointer).map_err(|e| {
            StorageError::ReadFailed(format!(
                "failed to read document at offset {} length {}: {e}",
                pointer.offset, pointer.length
            ))
        })?;
        bincode::deserialize(&bytes).map_err(|e| {
            StorageError::CorruptedData(format!(
                "failed to decode document at offset {} length {}: {e}",
                pointer.offset, pointer.length
            ))
            .into()
        })
    }

    /// Offset one past the last byte written.
    pub fn used_bytes(&self) -> u64 {
        self.append_cursor
    }

    /// Length of the mapping, or of the file when it is not mapped.
    pub fn mapped_len(&self) -> Result<usize> {
        let len =
            crate::storage::sidecars::mapped_or_file_len(self.mmap.as_deref(), &self.data_file)?;
        Ok(len as usize)
    }

    /// Fault the mapped file into the page cache. Does nothing when it is not mapped.
    pub fn warm_page_cache(&self) {
        if let Some(mmap) = self.mmap.as_ref() {
            warm_mmap(mmap);
        }
    }

    /// Flush the mapping and fsync the file.
    pub fn sync(&self) -> Result<()> {
        if let Some(mmap) = self.mmap.as_ref() {
            mmap.flush()?;
        }
        self.data_file.sync_all()?;
        Ok(())
    }

    fn read_bytes(&self, pointer: &EntryPointer) -> Result<Vec<u8>> {
        let offset = pointer.offset as usize;
        let length = pointer.length as usize;
        if let Some(mmap) = self.mmap.as_ref() {
            if let Some(end) = offset.checked_add(length).filter(|&end| end <= mmap.len()) {
                return Ok(mmap[offset..end].to_vec());
            }
        }

        let mut file = self.data_file.try_clone()?;
        let mut buffer = vec![0u8; length];
        file.seek(SeekFrom::Start(pointer.offset))?;
        file.read_exact(&mut buffer)?;
        Ok(buffer)
    }

    fn write_at(&mut self, offset: u64, bytes: &[u8]) -> Result<()> {
        if let Some(mmap) = self.mmap.as_mut() {
            let start = offset as usize;
            let end = start + bytes.len();
            mmap[start..end].copy_from_slice(bytes);
        } else {
            self.data_file.seek(SeekFrom::Start(offset))?;
            self.data_file.write_all(bytes)?;
        }
        Ok(())
    }
}

fn next_append_offset(index: &std::collections::HashMap<uuid::Uuid, EntryPointer>) -> u64 {
    index
        .values()
        .map(|pointer| pointer.offset + u64::from(pointer.length))
        .max()
        .unwrap_or(0)
}

fn initial_size(config: &CollectionConfig) -> u64 {
    if config.memory.use_mmap {
        config.memory.initial_mmap_size as u64
    } else {
        1024 * 1024
    }
}
