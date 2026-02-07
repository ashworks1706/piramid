use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;

use crate::error::Result;
use super::entry::WalEntry;

pub struct Wal {
    file: BufWriter<File>,
    path: PathBuf,
}

impl Wal {
    pub fn new(path: PathBuf) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        Ok(Wal {
            file: BufWriter::new(file),
            path,
        })
    }

    pub fn log(&mut self, entry: &WalEntry) -> Result<()> {
        let json = serde_json::to_string(entry)?;
        writeln!(self.file, "{}", json)?;
        self.file.flush()?;
        Ok(())
    }

    pub fn replay(&self) -> Result<Vec<WalEntry>> {
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();
        
        for line in reader.lines() {
            let line = line?;
            if !line.is_empty() {
                let entry: WalEntry = serde_json::from_str(&line)?;
                entries.push(entry);
            }
        }
        
        Ok(entries)
    }

    pub fn checkpoint(&mut self, timestamp: u64) -> Result<()> {
        self.log(&WalEntry::Checkpoint { timestamp })?;
        Ok(())
    }

    pub fn truncate(&mut self) -> Result<()> {
        drop(std::mem::replace(&mut self.file, BufWriter::new(File::create(&self.path)?)));
        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        self.file.flush()?;
        Ok(())
    }
}
