use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;

use crate::error::Result;
use super::entry::WalEntry;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct WalHeader {
    version: u32,
}

const WAL_VERSION: u32 = 1;

pub struct Wal {
    file: Option<BufWriter<File>>,
    path: PathBuf,
    pub next_seq: u64,
}

impl Wal {
    /// Create a WAL writer starting at the provided sequence.
    pub fn new(path: PathBuf, next_seq: u64) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        let mut wal = Wal {
            file: Some(BufWriter::new(file)),
            path,
            next_seq,
        };
        wal.ensure_header()?;
        Ok(wal)
    }
    
    /// Disabled WAL (noop) with a sequence counter for compatibility.
    pub fn disabled(path: PathBuf, next_seq: u64) -> Result<Self> {
        Ok(Wal {
            file: None,
            path,
            next_seq,
        })
    }

    /// Replay entries with seq greater than `min_seq`.
    pub fn replay(&self, min_seq: u64) -> Result<Vec<WalEntry>> {
        if self.file.is_none() {
            return Ok(Vec::new());
        }
        
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();
        
        for line in reader.lines() {
            let line = line?;
            if line.is_empty() {
                continue;
            }
            // Skip header if present (and validate version)
            if let Ok(header) = serde_json::from_str::<WalHeader>(&line) {
                if header.version != WAL_VERSION {
                    return Err(crate::error::PiramidError::other(format!(
                        "Unsupported WAL version {}, expected {}",
                        header.version, WAL_VERSION
                    )));
                }
                continue;
            }
            let entry: WalEntry = serde_json::from_str(&line)?;
            let entry_seq = match &entry {
                WalEntry::Insert { seq, .. }
                | WalEntry::Update { seq, .. }
                | WalEntry::Delete { seq, .. }
                | WalEntry::Checkpoint { seq, .. } => *seq,
            };
            if entry_seq <= min_seq {
                continue;
            }
            entries.push(entry);
        }
        
        Ok(entries)
    }

    // Log a new WAL entry. This method assigns the next sequence number to the entry, serializes it to JSON, and appends it to the WAL file. If the WAL is disabled (file is None), it simply increments the sequence number without writing anything.
    pub fn log(&mut self, entry: &mut WalEntry) -> Result<()> {
        match entry {
            WalEntry::Insert { seq, .. }
            | WalEntry::Update { seq, .. }
            | WalEntry::Delete { seq, .. }
            | WalEntry::Checkpoint { seq, .. } => {
                *seq = self.next_seq;
            }
        }
        if let Some(file) = &mut self.file {
            let json = serde_json::to_string(entry)?;
            writeln!(file, "{}", json)?;
            file.flush()?;
        }
        self.next_seq += 1;
        Ok(())
    }

    pub fn checkpoint(&mut self, timestamp: u64) -> Result<()> {
        let mut entry = WalEntry::Checkpoint { timestamp, seq: 0 };
        self.log(&mut entry)?;
        Ok(())
    }
    
    // Rotate the WAL file by closing the current one and starting a new, empty file. This is typically done after a checkpoint to prevent the WAL from growing indefinitely and to allow old entries to be safely discarded.
    pub fn rotate(&mut self) -> Result<()> {
        if self.file.is_none() {
            return Ok(());
        }
        // Drop current writer to release handle
        drop(self.file.take());
        // Open a fresh, truncated WAL file
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.path)?;
        file.sync_all()?;
        self.file = Some(BufWriter::new(file));
        self.ensure_header()?;
        Ok(())
    }
    
    pub fn flush(&mut self) -> Result<()> {
        if let Some(file) = &mut self.file {
            file.flush()?;
        }
        Ok(())
    }

    // Ensure the WAL file has a header with the correct version. If the file is new (size 0), we write the header. If the file already exists, we assume it has a valid header and do not modify it.
    fn ensure_header(&mut self) -> Result<()> {
        if self.file.is_none() {
            return Ok(());
        }
        let metadata = std::fs::metadata(&self.path)?;
        if metadata.len() == 0 {
            if let Some(writer) = &mut self.file {
                let header = WalHeader { version: WAL_VERSION };
                let json = serde_json::to_string(&header)?;
                writeln!(writer, "{}", json)?;
                writer.flush()?;
            }
        }
        Ok(())
    }
}
