// Vector storage using memory-mapped files
// Uses OS virtual memory to handle datasets efficiently
// 
// Architecture:
// - Vectors stored in mmap file (binary format)
// - Index maps UUID -> file offset
// - OS handles paging (loads only what's needed)

use memmap2::MmapMut;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use uuid::Uuid;

use crate::error::Result;
use crate::index::{HnswIndex, HnswConfig};
use super::wal::{Wal, WalEntry};
use super::utils::{VectorIndex, save_index, load_index, get_wal_path, ensure_file_size, create_mmap, grow_mmap_if_needed};
use crate::metadata::Metadata;
use crate::metrics::Metric;
use crate::search::SearchResult;
use crate::quantization::QuantizedVector;

use super::entry::VectorEntry;

// Vector storage engine with memory-mapped files and HNSW indexing
pub struct VectorStorage {
    data_file: File,
    mmap: Option<MmapMut>,
    index: HashMap<Uuid, VectorIndex>,
    hnsw_index: HnswIndex,
    path: String,
    wal: Wal,
}

impl VectorStorage {
    pub fn open(path: &str) -> Result<Self> {
        Self::with_hnsw(path, HnswConfig::default())
    }

    pub fn with_hnsw(path: &str, config: HnswConfig) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        ensure_file_size(&file, 1024 * 1024)?; // 1MB initial size
        let mmap = create_mmap(&file)?;

        // Load index from disk if exists
        let index = load_index(path)?;

        let mut hnsw_index = HnswIndex::new(config);
        
        // Initialize WAL
        let wal_path = get_wal_path(path);
        let wal = Wal::new(wal_path.into())?;
        
        // Replay WAL for crash recovery
        let wal_entries = wal.replay()?;
        if !wal_entries.is_empty() {
            // Create temporary storage to apply WAL entries
            let mut temp_storage = Self {
                data_file: file,
                mmap: Some(mmap),
                index,
                hnsw_index,
                path: path.to_string(),
                wal,
            };
            
            // Apply each WAL entry
            for entry in wal_entries {
                match entry {
                    WalEntry::Insert { id, vector, text, metadata } => {
                        let vec_entry = VectorEntry {
                            id,
                            vector: QuantizedVector::from_f32(&vector),
                            text,
                            metadata,
                        };
                        // Store without logging (already in WAL)
                        let _ = temp_storage.store_internal(vec_entry);
                    }
                    WalEntry::Update { id, vector, text, metadata } => {
                        // Delete old version and insert new
                        temp_storage.delete_internal(&id);
                        let vec_entry = VectorEntry {
                            id,
                            vector: QuantizedVector::from_f32(&vector),
                            text,
                            metadata,
                        };
                        let _ = temp_storage.store_internal(vec_entry);
                    }
                    WalEntry::Delete { id } => {
                        temp_storage.delete_internal(&id);
                    }
                    WalEntry::Checkpoint { .. } => {
                        // Skip checkpoints during recovery
                    }
                }
            }
            
            // Checkpoint after recovery to clear WAL
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            temp_storage.wal.checkpoint(timestamp)?;
            temp_storage.save_index()?;
            temp_storage.wal.truncate()?;
            
            return Ok(temp_storage);
        }
        
        // Rebuild HNSW from existing vectors
        if !index.is_empty() {
            let mut vectors: HashMap<Uuid, Vec<f32>> = HashMap::new();
            for (id, idx_entry) in &index {
                let offset = idx_entry.offset as usize;
                let length = idx_entry.length as usize;
                if offset + length <= mmap.len() {
                    let bytes = &mmap[offset..offset + length];
                    if let Ok(entry) = bincode::deserialize::<VectorEntry>(bytes) {
                        vectors.insert(*id, entry.get_vector());  // Dequantize
                    }
                }
            }
            
            for (id, vector) in &vectors {
                hnsw_index.insert(*id, vector, &vectors);
            }
        }
        
        Ok(Self {
            data_file: file,
            mmap: Some(mmap),
            index,
            hnsw_index,
            path: path.to_string(),
            wal,
        })
    }

    // Internal store without WAL logging (for recovery)
    fn store_internal(&mut self, entry: VectorEntry) -> Result<Uuid> {
        let id = entry.id;
        let bytes = bincode::serialize(&entry)?;

        let offset = self.index.values()
            .map(|idx| idx.offset + idx.length as u64)
            .max()
            .unwrap_or(0);

        let required_size = offset + bytes.len() as u64;
        grow_mmap_if_needed(&mut self.mmap, &self.data_file, required_size)?;
        
        let mmap = self.mmap.as_mut().unwrap();
        mmap[offset as usize..(offset as usize + bytes.len())]
            .copy_from_slice(&bytes);
        
        let index_entry = VectorIndex::new(offset, bytes.len() as u32);
        self.index.insert(id, index_entry);
        
        let vec_f32 = entry.get_vector();
        let mut vectors: HashMap<Uuid, Vec<f32>> = HashMap::new();
        for (vec_id, _) in &self.index {
            if let Some(vec_entry) = self.get(vec_id) {
                vectors.insert(*vec_id, vec_entry.get_vector());
            }
        }
        self.hnsw_index.insert(id, &vec_f32, &vectors);
        
        Ok(id)
    }

    // Internal delete without WAL logging (for recovery)
    fn delete_internal(&mut self, id: &Uuid) {
        self.index.remove(id);
        self.hnsw_index.remove(id);
    }

    pub fn store(&mut self, entry: VectorEntry) -> Result<Uuid> {
        // Log to WAL first
        let vector = entry.get_vector();
        self.wal.log(&WalEntry::Insert { 
            id: entry.id, 
            vector,
            text: entry.text.clone(),
            metadata: entry.metadata.clone() 
        })?;
        
        // Persist index to disk
        self.save_index()?;
        
        self.store_internal(entry)
    }

    pub fn store_batch(&mut self, entries: Vec<VectorEntry>) -> Result<Vec<Uuid>> {
        let mut ids = Vec::with_capacity(entries.len());
        
        // Log to WAL first
        for entry in &entries {
            let vector = entry.get_vector();
            self.wal.log(&WalEntry::Insert {
                id: entry.id,
                vector,
                text: entry.text.clone(),
                metadata: entry.metadata.clone()
            })?;
        }
        
        // Serialize all entries first
        let mut serialized: Vec<(Uuid, Vec<u8>)> = Vec::with_capacity(entries.len());
        for entry in &entries {
            let bytes = bincode::serialize(entry)?;
            serialized.push((entry.id, bytes));
        }
        
        // Calculate required space
        let current_offset = self.index.values()
            .map(|idx| idx.offset + idx.length as u64)
            .max()
            .unwrap_or(0);
        
        let total_bytes: u64 = serialized.iter().map(|(_, b)| b.len() as u64).sum();
        let required_size = current_offset + total_bytes;
        
        // Grow file if needed
        grow_mmap_if_needed(&mut self.mmap, &self.data_file, required_size)?;
        
        // Write all entries to mmap
        let mut offset = current_offset;
        let mmap = self.mmap.as_mut().unwrap();
        
        for (id, bytes) in &serialized {
            mmap[offset as usize..(offset as usize + bytes.len())]
                .copy_from_slice(bytes);
            
            let index_entry = VectorIndex {
                offset,
                length: bytes.len() as u32,
            };
            self.index.insert(*id, index_entry);
            ids.push(*id);
            
            offset += bytes.len() as u64;
        }
        
        // Persist index once
        self.save_index()?;
        
        // Build vectors map for HNSW
        let mut vectors: HashMap<Uuid, Vec<f32>> = HashMap::new();
        for (vec_id, _) in &self.index {
            if let Some(vec_entry) = self.get(vec_id) {
                vectors.insert(*vec_id, vec_entry.get_vector());
            }
        }
        
        // Insert into HNSW index
        for entry in entries {
            let vec_f32 = entry.get_vector();
            self.hnsw_index.insert(entry.id, &vec_f32, &vectors);
        }
        
        Ok(ids)
    }

    fn save_index(&self) -> Result<()> {
        save_index(&self.path, &self.index)
    }

    pub fn get(&self, id: &Uuid) -> Option<VectorEntry> {
        let index_entry = self.index.get(id)?;
        let offset = index_entry.offset as usize;
        let length = index_entry.length as usize;
        let bytes = &self.mmap.as_ref().unwrap()[offset..offset + length];
        bincode::deserialize(bytes).ok()
    }

    pub fn search(&self, query: &[f32], k: usize, metric: Metric) -> Vec<SearchResult> {
        let mut vectors: HashMap<Uuid, Vec<f32>> = HashMap::new();
        for (id, _) in &self.index {
            if let Some(entry) = self.get(id) {
                vectors.insert(*id, entry.get_vector());  // Dequantize
            }
        }
        
        let neighbor_ids = self.hnsw_index.search(query, k, k * 2, &vectors);
        
        let mut results = Vec::new();
        for id in neighbor_ids {
            if let Some(entry) = self.get(&id) {
                let vec = entry.get_vector();  // Dequantize
                let score = metric.calculate(query, &vec);
                results.push(SearchResult {
                    id,
                    score,
                    text: entry.text,
                    vector: vec,
                    metadata: entry.metadata.clone(),
                });
            }
        }
        results
    }

    // Batch search - search multiple queries in parallel
    // Returns results for each query in the same order
    pub fn search_batch(&self, queries: &[Vec<f32>], k: usize, metric: Metric) -> Vec<Vec<SearchResult>> {
        use rayon::prelude::*;
        
        queries
            .par_iter()
            .map(|query| self.search(query, k, metric))
            .collect()
    }


    // Get number of vectors
    pub fn count(&self) -> usize {
        self.index.len()
    }

    pub fn index(&self) -> &HnswIndex {
        &self.hnsw_index
    }

    pub fn get_vectors(&self) -> HashMap<Uuid, Vec<f32>> {
        let mut vectors = HashMap::new();
        for (id, _) in &self.index {
            if let Some(entry) = self.get(id) {
                vectors.insert(*id, entry.get_vector());  // Dequantize
            }
        }
        vectors
    }

    pub fn search_with_filter(
        &self,
        query: &[f32],
        k: usize,
        metric: Metric,
        filter: Option<&crate::query::Filter>,
    ) -> Vec<SearchResult> {
        match filter {
            Some(f) => crate::search::filtered_search(self, query, k, metric, f),
            None => self.search(query, k, metric),
        }
    }

    pub fn delete(&mut self, id: &Uuid) -> Result<bool> {
        if self.index.contains_key(id) {
            // Log to WAL
            self.wal.log(&WalEntry::Delete { id: *id })?;
            
            self.delete_internal(id);
            self.save_index()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    pub fn update_metadata(&mut self, id: &Uuid, metadata: Metadata) -> Result<bool> {
        if let Some(entry) = self.get(id) {
            let vector = entry.get_vector();
            
            // Log to WAL
            self.wal.log(&WalEntry::Update {
                id: *id,
                vector,
                text: entry.text.clone(),
                metadata: metadata.clone()
            })?;
            
            let mut entry = entry;
            entry.metadata = metadata;
            self.delete(id)?;
            self.store(entry)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    pub fn update_vector(&mut self, id: &Uuid, vector: Vec<f32>) -> Result<bool> {
        if let Some(entry) = self.get(id) {
            // Log to WAL
            self.wal.log(&WalEntry::Update {
                id: *id,
                vector: vector.clone(),
                text: entry.text.clone(),
                metadata: entry.metadata.clone()
            })?;
            
            let mut entry = entry;
            entry.vector = QuantizedVector::from_f32(&vector);  // Quantize new vector
            self.delete(id)?;
            
            // Build vectors map for HNSW
            let mut vectors: HashMap<Uuid, Vec<f32>> = HashMap::new();
            for (vec_id, _) in &self.index {
                if let Some(vec_entry) = self.get(vec_id) {
                    vectors.insert(*vec_id, vec_entry.get_vector());  // Dequantize
                }
            }
            
            self.store(entry)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    pub fn get_all(&self) -> Vec<VectorEntry> {
        let mut all_entries = Vec::new();
        for (id, _) in &self.index {
            if let Some(entry) = self.get(id) {
                all_entries.push(entry);
            }
        }
        all_entries
    }

    pub fn checkpoint(&mut self) -> Result<()> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        self.wal.checkpoint(timestamp)?;
        self.save_index()?;
        self.wal.truncate()?;
        
        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        self.wal.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_store_and_retrieve() {
        let _ = std::fs::remove_file("test.db");
        let _ = std::fs::remove_file("test.index.db");
        
        let mut storage = VectorStorage::open("test.db").unwrap();
        let entry = VectorEntry::new(vec![1.0, 2.0, 3.0], "test".to_string());
        let id = storage.store(entry).unwrap();
        
        let retrieved = storage.get(&id).unwrap();
        assert_eq!(retrieved.text, "test");
        assert_eq!(retrieved.get_vector(), vec![1.0, 2.0, 3.0]);
        
        std::fs::remove_file("test.db").unwrap();
        std::fs::remove_file("test.index.db").unwrap();
    }

    #[test]
    fn test_persistence() {
        let _ = std::fs::remove_file("test_persist.db");
        let _ = std::fs::remove_file("test_persist.index.db");
        
        let id1;
        let id2;
        
        {
            let mut storage = VectorStorage::open("test_persist.db").unwrap();
            let e1 = VectorEntry::new(vec![1.0, 2.0], "first".to_string());
            let e2 = VectorEntry::new(vec![3.0, 4.0], "second".to_string());
            id1 = storage.store(e1).unwrap();
            id2 = storage.store(e2).unwrap();
        }
        
        {
            let storage = VectorStorage::open("test_persist.db").unwrap();
            assert_eq!(storage.count(), 2);
            assert_eq!(storage.get(&id1).unwrap().text, "first");
            assert_eq!(storage.get(&id2).unwrap().text, "second");
        }
        
        std::fs::remove_file("test_persist.db").unwrap();
        std::fs::remove_file("test_persist.index.db").unwrap();
    }

    #[test]
    fn test_search() {
        let _ = std::fs::remove_file("test_search.db");
        let _ = std::fs::remove_file("test_search.index.db");
        
        let mut storage = VectorStorage::open("test_search.db").unwrap();
        
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
        
        let query = vec![1.0, 0.0, 0.0];
        let results = storage.search(&query, 2, Metric::Cosine);
        
        assert_eq!(results.len(), 2);
        
        std::fs::remove_file("test_search.db").unwrap();
        std::fs::remove_file("test_search.index.db").unwrap();
    }

    #[test]
    fn test_batch_search() {
        let _ = std::fs::remove_file("test_batch_search.db");
        let _ = std::fs::remove_file("test_batch_search.index.db");
        
        let mut storage = VectorStorage::open("test_batch_search.db").unwrap();
        
        // Insert test vectors
        for i in 0..10 {
            let vec = vec![i as f32, 0.0, 0.0];
            let entry = VectorEntry::new(vec, format!("vec{}", i));
            storage.store(entry).unwrap();
        }
        
        // Batch search with multiple queries
        let queries = vec![
            vec![0.0, 0.0, 0.0],
            vec![5.0, 0.0, 0.0],
            vec![9.0, 0.0, 0.0],
        ];
        
        let results = storage.search_batch(&queries, 2, Metric::Euclidean);
        
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].len(), 2);
        assert_eq!(results[1].len(), 2);
        assert_eq!(results[2].len(), 2);
        
        std::fs::remove_file("test_batch_search.db").unwrap();
        std::fs::remove_file("test_batch_search.index.db").unwrap();
    }
}
