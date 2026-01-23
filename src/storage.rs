use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use uuid::Uuid;

use crate::metrics::SimilarityMetric;
use crate::error::Result;
use crate::metadata::Metadata;
use crate::query::Filter;
use crate::search::SearchResult;

/// A single vector entry stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorEntry {
    pub id: Uuid,
    pub vector: Vec<f32>,
    pub text: String,
    #[serde(default)]
    pub metadata: Metadata,
}

impl VectorEntry {
    pub fn new(vector: Vec<f32>, text: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            vector,
            text,
            metadata: Metadata::new(),
        }
    }

    pub fn with_metadata(vector: Vec<f32>, text: String, metadata: Metadata) -> Self {
        Self {
            id: Uuid::new_v4(),
            vector,
            text,
            metadata,
        }
    }
}

/// Vector storage engine
/// 
/// Keeps all vectors in RAM with persistence to disk using bincode serialization.
/// 
/// # Note
/// Current implementation does full file rewrites on each mutation.
/// Phase 9.5 will add WAL for durability and better performance.
pub struct VectorStorage {
    file: File,
    vectors: HashMap<Uuid, VectorEntry>,
}

impl VectorStorage {
    pub fn open(path: &str) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        let mut storage = Self {
            file,
            vectors: HashMap::new(),
        };

        storage.load()?;
        Ok(storage)
    }

    pub fn store(&mut self, entry: VectorEntry) -> Result<Uuid> {
        let id = entry.id;
        self.vectors.insert(id, entry);
        self.save()?;
        Ok(id)
    }

    pub fn get(&self, id: &Uuid) -> Option<VectorEntry> {
        self.vectors.get(id).cloned()
    }

    pub fn get_all(&self) -> Vec<&VectorEntry> {
        self.vectors.values().collect()
    }

    pub fn count(&self) -> usize {
        self.vectors.len()
    }

    /// Brute-force similarity search
    /// 
    /// # Performance
    /// O(n) complexity - compares query against all vectors.
    /// Suitable for <10k vectors. Phase 9 will add HNSW indexing.
    pub fn search(&self, query: &[f32], k: usize, metric: SimilarityMetric) -> Vec<SearchResult> {
        self.search_with_filter(query, k, metric, None)
    }

    /// Similarity search with metadata filtering
    pub fn search_with_filter(
        &self,
        query: &[f32],
        k: usize,
        metric: SimilarityMetric,
        filter: Option<&Filter>,
    ) -> Vec<SearchResult> {
        let mut results: Vec<SearchResult> = self.vectors
            .values()
            .filter(|entry| {
                filter.map_or(true, |f| f.matches(&entry.metadata))
            })
            .map(|entry| {
                let score = metric.calculate(query, &entry.vector);
                SearchResult::new(
                    entry.id,
                    score,
                    entry.text.clone(),
                    entry.vector.clone(),
                    entry.metadata.clone(),
                )
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(k);
        results
    }

    pub fn delete(&mut self, id: &Uuid) -> Result<bool> {
        let existed = self.vectors.remove(id).is_some();
        if existed {
            self.save()?;
        }
        Ok(existed)
    }

    pub fn update_metadata(&mut self, id: &Uuid, metadata: Metadata) -> Result<bool> {
        if let Some(entry) = self.vectors.get_mut(id) {
            entry.metadata = metadata;
            self.save()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn update_vector(&mut self, id: &Uuid, vector: Vec<f32>) -> Result<bool> {
        if let Some(entry) = self.vectors.get_mut(id) {
            entry.vector = vector;
            self.save()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn load(&mut self) -> Result<()> {
        self.file.seek(SeekFrom::Start(0))?;
        let mut buf = Vec::new();
        self.file.read_to_end(&mut buf)?;

        if buf.is_empty() {
            return Ok(());
        }

        self.vectors = bincode::deserialize(&buf)?;
        Ok(())
    }

    fn save(&mut self) -> Result<()> {
        let data = bincode::serialize(&self.vectors)?;
        self.file.seek(SeekFrom::Start(0))?;
        self.file.set_len(0)?;
        self.file.write_all(&data)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_and_retrieve() {
        let _ = std::fs::remove_file("test.db");
        let mut storage = VectorStorage::open("test.db").unwrap();

        let vector = vec![0.1, 0.2, 0.3];
        let entry = VectorEntry::new(vector.clone(), "hello".to_string());
        let id = storage.store(entry).unwrap();

        let retrieved = storage.get(&id).unwrap();
        assert_eq!(retrieved.vector, vector);
        assert_eq!(retrieved.text, "hello");

        std::fs::remove_file("test.db").unwrap();
    }

    #[test]
    fn test_persistence() {
        let _ = std::fs::remove_file("test2.db");

        let id1;
        let id2;

        {
            let mut storage = VectorStorage::open("test2.db").unwrap();
            let e1 = VectorEntry::new(vec![1.0, 2.0], "first".to_string());
            let e2 = VectorEntry::new(vec![3.0, 4.0], "second".to_string());
            id1 = storage.store(e1).unwrap();
            id2 = storage.store(e2).unwrap();
        }

        {
            let storage = VectorStorage::open("test2.db").unwrap();
            assert_eq!(storage.count(), 2);
            assert_eq!(storage.get(&id1).unwrap().text, "first");
            assert_eq!(storage.get(&id2).unwrap().text, "second");
        }

        std::fs::remove_file("test2.db").unwrap();
    }
}
