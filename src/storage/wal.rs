use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use serde_json::Value;

use crate::error::Result;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum WalEntry{
    Insert { id : Uuid, vector : Vec<f32>, metadata: HashMap<String, Value> },
    Update { id: Uuid, vector : Vec<f32>, metadata: HashMap<String, Value> },
    Delete { id: Uuid },
    Checkpoint { timestamp: u64}
}

pub struct Wal {
    file: BufWriter<File>,
    path: PathBuf,
}

impl Wal {
/// Creates a new WAL instance by opening the specified file path for appending. If the file does
/// not exist, it will be created. The WAL will be used to log operations for durability and
/// recovery.
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
/// Logs an operation to the WAL file by serializing the WalEntry to JSON and writing it as a new
/// line.
    pub fn log(&mut self, entry: &WalEntry) -> Result<()> {
        let json = serde_json::to_string(entry)?;
        writeln!(self.file, "{}", json)?;
        self.file.flush()?;
        Ok(())
    }
/// Reads the WAL file and returns a vector of WalEntry for replaying the operations.
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
