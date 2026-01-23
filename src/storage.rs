//  `use` brings types into scope. `serde` is THE serialization library.
// Derive macros auto-generate code - Serialize/Deserialize let us save to disk.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;  
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};  // Traits for file I/O
use uuid::Uuid;

use crate::metrics::SimilarityMetric;  // `crate::` means "from this project"
use crate::error::Result;
use crate::metadata::Metadata;
use crate::query::Filter;
use crate::search::SearchResult;

// A single vector entry - what you store in the database
//  #[derive(...)] auto-generates trait implementations
// - Debug: lets you print with {:?}
// - Clone: lets you duplicate with .clone()
// - Serialize/Deserialize: serde magic for saving/loading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorEntry {
    pub id: Uuid,
    pub vector: Vec<f32>,  // Vec = growable array, f32 = 32-bit float
    pub text: String,
    #[serde(default)]      // If missing when loading, use Default (empty HashMap)
    pub metadata: Metadata,
}

//  `impl` block adds methods to a struct
impl VectorEntry {
    // `Self` = the type we're implementing (VectorEntry)
    pub fn new(vector: Vec<f32>, text: String) -> Self {
        Self {
            id: Uuid::new_v4(),  // v4 = random UUID
            vector,              // shorthand for `vector: vector`
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

/// The storage engine. Keeps all vectors in RAM, syncs to disk.
pub struct VectorStorage {
    file: File,                           // no `pub` = private field
    vectors: HashMap<Uuid, VectorEntry>,  // our in-memory "database"
}

impl VectorStorage {
    //  no `self` param = "static method", called as VectorStorage::open()
    pub fn open(path: &str) -> Result<Self> {
        //  chain methods to configure, then .open()
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)  // create if doesn't exist
            .open(path)?;  // `?` = return early if error

        let mut storage = Self {
            file,
            vectors: HashMap::new(),
        };

        storage.load()?;
        Ok(storage)  //  last expression without ; is the return value
    }

    // `&mut self` = we're modifying the storage
    pub fn store(&mut self, entry: VectorEntry) -> Result<Uuid> {
        let id = entry.id;  // copy the id before entry moves into the HashMap
        self.vectors.insert(id, entry);  // entry is "moved" here, can't use it after
        self.save()?;
        Ok(id)
    }

    // `&self` = we're just reading, not modifying
    pub fn get(&self, id: &Uuid) -> Option<VectorEntry> {
        // .cloned() because HashMap stores the value, we return a copy
        self.vectors.get(id).cloned()
    }

    pub fn get_all(&self) -> Vec<&VectorEntry> {
        self.vectors.values().collect()
    }

    pub fn count(&self) -> usize {
        self.vectors.len()
    }

    /// Brute-force search: compare query against ALL vectors.
    /// O(n) complexity - fine for <10k vectors, need indexing for more.
    pub fn search(&self, query: &[f32], k: usize, metric: SimilarityMetric) -> Vec<SearchResult> {
        self.search_with_filter(query, k, metric, None)
    }

    /// Same as search(), but filter by metadata first
    pub fn search_with_filter(
        &self,
        query: &[f32],        // &[f32] = slice, a view into any contiguous f32s
        k: usize,             // usize = unsigned integer, size depends on platform
        metric: SimilarityMetric,
        filter: Option<&Filter>,  // Option = might be None or Some(value)
    ) -> Vec<SearchResult> {
        // Rust iterators: lazy, chainable, zero-cost abstractions
        let mut results: Vec<SearchResult> = self.vectors
            .values()                    // iterate over HashMap values
            .filter(|entry| {            // |entry| is a closure (lambda)
                // map_or(default, fn): if None use default, if Some apply fn
                filter.map_or(true, |f| f.matches(&entry.metadata))
            })
            .map(|entry| {               // transform each entry into SearchResult
                let score = metric.calculate(query, &entry.vector);
                SearchResult::new(
                    entry.id,
                    score,
                    entry.text.clone(),   // clone because we're borrowing entry
                    entry.vector.clone(),
                    entry.metadata.clone(),
                )
            })
            .collect();  // consume iterator, collect into Vec

        // partial_cmp for floats (might be NaN), unwrap_or handles that edge case
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // Return top k results
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

        // bincode = fast binary serialization, much smaller than JSON
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

    #[test] //tests 
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
