use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use uuid::Uuid;

use crate::metrics::Metric;
use crate::error::Result;
use crate::metadata::Metadata;
use crate::query::Filter;
use crate::search::SearchResult;
use crate::index::{HnswIndex, HnswConfig};

// A single vector entry stored in the database
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

// Vector storage engine with HNSW indexing for fast approximate nearest neighbor search.
pub struct VectorStorage {
    file: File,
    path: String, // store the file path for index file naming 
    vectors: HashMap<Uuid, VectorEntry>,
    hnsw_index: HnswIndex, // HNSW index (required)
}

impl VectorStorage {
    // Get reference to the HNSW index
    pub fn index(&self) -> &HnswIndex {
        &self.hnsw_index
    }

    // Get reference to all vectors
    pub fn get_vectors(&self) -> &HashMap<Uuid, VectorEntry> {
        &self.vectors
    }
    // Internal helper to create vector map (reused across insert/update/rebuild)
    fn create_vector_map_internal(&self) -> HashMap<Uuid, Vec<f32>> {
        self.vectors
            .iter()
            .map(|(id, entry)| (*id, entry.vector.clone()))
            .collect()
    }

    /// Get the HNSW index file path from the database file path
    /// Converts "data/docs.db" -> "data/.hnsw.db"
    /// Converts "docs.db" -> ".hnsw.db"
    /// This keeps all *.db files covered by a single .gitignore rule
    fn index_path(&self) -> String {
        let path = std::path::Path::new(&self.path);
        if let Some(parent) = path.parent() {
            let dir = parent.to_str().unwrap_or(".");
            if dir.is_empty() {
                ".hnsw.db".to_string()
            } else {
                format!("{}/.hnsw.db", dir)
            }
        } else {
            ".hnsw.db".to_string()
        }
    }

    // Create storage with default HNSW configuration
    pub fn open(path: &str) -> Result<Self> {
        Self::with_hnsw(path, HnswConfig::default())
    }

    // Create storage with custom HNSW configuration
    pub fn with_hnsw(path: &str, config: HnswConfig) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        let mut storage = Self {
            file,
            path: path.to_string(),
            vectors: HashMap::new(),
            hnsw_index: HnswIndex::new(config),
        };

        storage.load()?;

        // try to load index from disk 
        let index_path = storage.index_path();
        let index_loaded = if let Ok(mut index_file) = File::open(&index_path){
            let mut index_data = Vec::new();
            if index_file.read_to_end(&mut index_data).is_ok(){
                if let Ok(loaded_index) = bincode::deserialize::<HnswIndex>(&index_data){
                    storage.hnsw_index = loaded_index;
                    true
                } else {
                    false // corrupted index file 
                }
            } else {
                false // can't read the file 
            }
        } else {
            false // file doesn't exist 
        };
        
        // if index not loaded, rebuild from vectors
        if !index_loaded && !storage.vectors.is_empty(){
            storage.rebuild_index();
        }
        
        Ok(storage)
    }

    // Rebuild the HNSW index from all vectors
    pub fn rebuild_index(&mut self) {
        let config = HnswConfig::default(); // Get current config or use default
        let mut new_index = HnswIndex::new(config);
        
        let vector_map = self.create_vector_map_internal();

        for (id, entry) in &self.vectors {
            new_index.insert(*id, &entry.vector, &vector_map);
        }

        self.hnsw_index = new_index;
    }

    pub fn store(&mut self, entry: VectorEntry) -> Result<Uuid> {
        let id = entry.id;
        
        // Update HNSW index
        let vector_map = self.create_vector_map_internal();
        self.hnsw_index.insert(id, &entry.vector, &vector_map);
        
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

    // Search for k nearest neighbors using HNSW index
    // Time complexity: O(log n) approximate search
    // 
    // Convenience wrapper around `search::vector_search()`
    pub fn search(&self, query: &[f32], k: usize, metric: Metric) -> Vec<SearchResult> {
        crate::search::vector_search(self, query, k, metric)
    }

    // Search with metadata filtering
    // 
    // Convenience wrapper around `search::filtered_search()`
    pub fn search_with_filter(
        &self,
        query: &[f32],
        k: usize,
        metric: Metric,
        filter: Option<&Filter>,
    ) -> Vec<SearchResult> {
        match filter {
            Some(f) => crate::search::filtered_search(self, query, k, metric, f),
            None => crate::search::vector_search(self, query, k, metric),
        }
    }

    pub fn delete(&mut self, id: &Uuid) -> Result<bool> {
        let existed = self.vectors.remove(id).is_some();
        
        // Remove from HNSW index
        if existed {
            self.hnsw_index.remove(id);
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
            entry.vector = vector.clone();
            
            // Update HNSW index: remove old entry and re-insert with new vector
            self.hnsw_index.remove(id);
            let vector_map = self.create_vector_map_internal();
            self.hnsw_index.insert(*id, &vector, &vector_map);
            
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
        self.file.sync_all()?;

        // Save index to seperate .db.* file 
        if let Ok(index_data) = bincode::serialize(&self.hnsw_index){
            if let Ok(mut index_file) = File::create(&self.index_path()){
                let _ = index_file.write_all(&index_data);
                let _ = index_file.sync_all();
            }
        }
        // if index save fails, we continue (index can be rebuilt on load)
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

    #[test]
    fn test_hnsw_search() {
        let _ = std::fs::remove_file("test_hnsw.db");
        let _ = std::fs::remove_file(".hnsw.db");
        
        // Create storage with default HNSW config
        let mut storage = VectorStorage::open("test_hnsw.db").unwrap();

        // Insert test vectors
        let vectors = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
            vec![0.9, 0.1, 0.0],
        ];

        for (i, vec) in vectors.iter().enumerate() {
            let entry = VectorEntry::new(vec.clone(), format!("vec{}", i));
            storage.store(entry).unwrap();
        }

        // Search with HNSW
        let query = vec![1.0, 0.0, 0.0];
        let results = storage.search(&query, 2, Metric::Cosine);

        assert_eq!(results.len(), 2);
        assert!(results[0].text == "vec0" || results[0].text == "vec3"); // Should find similar vectors

        std::fs::remove_file("test_hnsw.db").unwrap();
        let _ = std::fs::remove_file(".hnsw.db");
    }

    #[test]
    fn test_hnsw_with_filter() {
        let _ = std::fs::remove_file("test_filter.db");
        let _ = std::fs::remove_file("test_filter.db.hnsw");
        
        let mut storage = VectorStorage::open("test_filter.db").unwrap();

        // Insert vectors with metadata
        let e1 = VectorEntry::with_metadata(
            vec![1.0, 0.0],
            "doc1".to_string(),
            crate::metadata::metadata([("category", "A".into())])
        );
        let e2 = VectorEntry::with_metadata(
            vec![0.9, 0.1],
            "doc2".to_string(),
            crate::metadata::metadata([("category", "B".into())])
        );
        
        storage.store(e1).unwrap();
        storage.store(e2).unwrap();

        // Search with filter
        let filter = Filter::new().eq("category", "A");
        let query = vec![1.0, 0.0];
        let results = storage.search_with_filter(&query, 5, Metric::Cosine, Some(&filter));

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].text, "doc1");

        std::fs::remove_file("test_filter.db").unwrap();
        let _ = std::fs::remove_file("test_filter.db.hnsw");
    }
}
