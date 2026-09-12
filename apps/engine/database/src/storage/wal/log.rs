//! Write-ahead log: one JSON entry per line, each with a monotonic sequence number.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;

use super::entry::WalEntry;
use piramid_core::error::Result;
#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct WalHeader {
    version: u32,
}

const WAL_VERSION: u32 = 1;

/// A collection's write-ahead log file, or a stand-in that writes nothing when logging is off.
pub struct Wal {
    file: Option<BufWriter<File>>,
    path: PathBuf,
    /// Sequence number the next logged entry receives.
    pub next_seq: u64,
    /// Calls fsync after every entry. Without it a write reaches the kernel and no further.
    sync_on_write: bool,
}

impl Wal {
    /// Create a WAL writer starting at the provided sequence.
    ///
    /// The sync_on_write flag decides whether an entry is durable when log returns. With it off
    /// the entry sits in the kernel buffer.
    pub fn new(path: PathBuf, next_seq: u64, sync_on_write: bool) -> Result<Self> {
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        let mut wal = Wal {
            file: Some(BufWriter::new(file)),
            path,
            next_seq,
            sync_on_write,
        };
        wal.ensure_header()?;
        Ok(wal)
    }

    /// A WAL that writes nothing, for a disabled wal.
    pub fn disabled(path: PathBuf, next_seq: u64) -> Result<Self> {
        Ok(Wal {
            file: None,
            path,
            next_seq,
            sync_on_write: false,
        })
    }

    /// Bytes currently on disk, or None when logging is disabled. Errors when the log file
    /// cannot be inspected.
    pub fn size_bytes(&self) -> Result<Option<u64>> {
        if self.file.is_none() {
            return Ok(None);
        }
        Ok(Some(std::fs::metadata(&self.path)?.len()))
    }

    /// Replay entries with a seq greater than min_seq.
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
            if let Ok(header) = serde_json::from_str::<WalHeader>(&line) {
                if header.version != WAL_VERSION {
                    return Err(piramid_core::error::PiramidError::other(format!(
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

    /// Appends an entry, assigning it the next sequence number.
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
            writeln!(file, "{json}")?;
            file.flush()?;
            if self.sync_on_write {
                // flush only drains the BufWriter into the kernel, so sync_all follows it.
                file.get_ref().sync_all()?;
            }
        }
        self.next_seq += 1;
        Ok(())
    }

    /// Append a checkpoint entry stamped with timestamp.
    pub fn checkpoint(&mut self, timestamp: u64) -> Result<()> {
        let mut entry = WalEntry::Checkpoint { timestamp, seq: 0 };
        self.log(&mut entry)?;
        Ok(())
    }

    /// Truncates the log and starts again. Safe only after a checkpoint.
    pub fn rotate(&mut self) -> Result<()> {
        if self.file.is_none() {
            return Ok(());
        }
        drop(self.file.take());
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

    /// Drain buffered entries to the kernel without fsync.
    pub fn flush(&mut self) -> Result<()> {
        if let Some(file) = &mut self.file {
            file.flush()?;
        }
        Ok(())
    }

    /// Writes the version header if the file is empty.
    fn ensure_header(&mut self) -> Result<()> {
        if self.file.is_none() {
            return Ok(());
        }
        let metadata = std::fs::metadata(&self.path)?;
        if metadata.len() == 0 {
            if let Some(writer) = &mut self.file {
                let header = WalHeader {
                    version: WAL_VERSION,
                };
                let json = serde_json::to_string(&header)?;
                writeln!(writer, "{json}")?;
                writer.flush()?;
            }
        }
        Ok(())
    }
}
