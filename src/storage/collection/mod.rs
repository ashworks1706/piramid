// Collection module - modular organization
//
// This module now uses a modular structure:
// - storage.rs: Core data structure and basic accessors
// - builder.rs: Initialization and recovery logic
// - operations.rs: CRUD operations (insert, delete, update)
// - persistence.rs: Disk operations and checkpointing

mod storage;
mod operations;
mod builder;
mod cache;
mod persistence;

pub use storage::Collection;
pub use builder::CollectionBuilder;

#[derive(Clone)]
pub struct CollectionOpenOptions {
    pub config: crate::config::CollectionConfig,
}

impl Default for CollectionOpenOptions {
    fn default() -> Self {
        Self {
            config: crate::config::CollectionConfig::default(),
        }
    }
}

impl From<crate::config::CollectionConfig> for CollectionOpenOptions {
    fn from(config: crate::config::CollectionConfig) -> Self {
        Self { config }
    }
}

use uuid::Uuid;
use crate::error::Result;
use crate::metadata::Metadata;
use crate::metrics::Metric;
use crate::search::Hit;
use crate::storage::document::Document;
use std::collections::HashMap;

// Public API implementation
impl Collection {
    pub fn open(path: &str) -> Result<Self> {
        CollectionBuilder::open(path, CollectionOpenOptions::default())
    }

    pub fn open_with_options(path: &str, options: CollectionOpenOptions) -> Result<Self> {
        CollectionBuilder::open(path, options)
    }

    pub fn get(&self, id: &Uuid) -> Option<Document> {
        operations::get(self, id)
    }

    pub fn insert(&mut self, entry: Document) -> Result<Uuid> {
        operations::insert(self, entry)
    }

    pub fn insert_batch(&mut self, entries: Vec<Document>) -> Result<Vec<Uuid>> {
        operations::insert_batch(self, entries)
    }
    
    pub fn upsert(&mut self, entry: Document) -> Result<Uuid> {
        operations::upsert(self, entry)
    }

    pub fn delete(&mut self, id: &Uuid) -> Result<bool> {
        operations::delete(self, id)
    }

    pub fn delete_batch(&mut self, ids: &[Uuid]) -> Result<usize> {
        operations::delete_batch(self, ids)
    }
    
    
    pub fn update_metadata(&mut self, id: &Uuid, metadata: Metadata) -> Result<bool> {
        operations::update_metadata(self, id, metadata)
    }
    
    pub fn update_vector(&mut self, id: &Uuid, vector: Vec<f32>) -> Result<bool> {
        operations::update_vector(self, id, vector)
    }

    pub fn search(&self, query: &[f32], k: usize, metric: Metric, params: crate::search::SearchParams) -> Vec<Hit> {
        let mut effective_params = params;
        if matches!(effective_params.mode, crate::config::ExecutionMode::Auto) {
            effective_params.mode = self.config().execution;
        }
        if effective_params.filter_overfetch_override.is_none() {
            effective_params.filter_overfetch_override = Some(self.config.search.filter_overfetch);
        }
        crate::search::search_collection(self, query, k, metric, effective_params)
    }

    pub fn search_batch(&self, queries: &[Vec<f32>], k: usize, metric: Metric) -> Vec<Vec<Hit>> {
        let params = crate::search::SearchParams {
            mode: self.config().execution,
            filter: None,
            filter_overfetch_override: None,
            search_config_override: None,
        };
        crate::search::search_batch_collection(self, queries, k, metric, params)
    }

    pub fn search_batch_with_params(&self, queries: &[Vec<f32>], k: usize, metric: Metric, params: crate::search::SearchParams) -> Vec<Vec<Hit>> {
        let mut effective_params = params;
        if matches!(effective_params.mode, crate::config::ExecutionMode::Auto) {
            effective_params.mode = self.config().execution;
        }
        if effective_params.search_config_override.is_none() {
            effective_params.search_config_override = Some(self.config.search);
        }
        crate::search::search_batch_collection(self, queries, k, metric, effective_params)
    }

    pub fn get_vectors(&self) -> &HashMap<Uuid, Vec<f32>> {
        self.vectors_view()
    }

    pub fn checkpoint(&mut self) -> Result<()> {
        persistence::checkpoint(self)
    }

    pub fn flush(&mut self) -> Result<()> {
        persistence::flush(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::Metric;

    #[test]
    fn test_basic_store_and_retrieve() {
        let test_path = ".piramid/tests/test_basic.db";
        let test_index = ".piramid/tests/test_basic.db.index.db";
        let test_wal = ".piramid/tests/test_basic.db.wal.db";
        let test_vecindex = ".piramid/tests/test_basic.db.vecindex.db";
        let test_meta = ".piramid/tests/test_basic.db.metadata.db";
        
        let _ = std::fs::remove_file(test_path);
        let _ = std::fs::remove_file(test_index);
        let _ = std::fs::remove_file(test_wal);
        let _ = std::fs::remove_file(test_vecindex);
        let _ = std::fs::remove_file(test_meta);
        
        let mut storage = Collection::open(test_path).unwrap();
        let entry = Document::new(vec![1.0, 2.0, 3.0], "test".to_string());
        let id = storage.insert(entry).unwrap();
        
        let retrieved = storage.get(&id).unwrap();
        assert_eq!(retrieved.text, "test");
        assert_eq!(retrieved.get_vector(), vec![1.0, 2.0, 3.0]);
        
        drop(storage);
        std::fs::remove_file(test_path).unwrap();
        std::fs::remove_file(test_index).unwrap();
        let _ = std::fs::remove_file(test_wal);
        let _ = std::fs::remove_file(test_vecindex);
        let _ = std::fs::remove_file(test_meta);
    }

    #[test]
    fn test_persistence() {
        let test_path = ".piramid/tests/test_persist.db";
        let test_index = ".piramid/tests/test_persist.db.index.db";
        let test_wal = ".piramid/tests/test_persist.db.wal.db";
        let test_vecindex = ".piramid/tests/test_persist.db.vecindex.db";
        let test_meta = ".piramid/tests/test_persist.db.metadata.db";
        
        let _ = std::fs::remove_file(test_path);
        let _ = std::fs::remove_file(test_index);
        let _ = std::fs::remove_file(test_wal);
        let _ = std::fs::remove_file(test_vecindex);
        let _ = std::fs::remove_file(test_meta);
        
        let id1;
        let id2;
        
        {
            let mut storage = Collection::open(test_path).unwrap();
            let e1 = Document::new(vec![1.0, 2.0], "first".to_string());
            let e2 = Document::new(vec![3.0, 4.0], "second".to_string());
            id1 = storage.insert(e1).unwrap();
            id2 = storage.insert(e2).unwrap();
        }
        
        {
            let storage = Collection::open(test_path).unwrap();
            assert_eq!(storage.count(), 2);
            assert_eq!(storage.get(&id1).unwrap().text, "first");
            assert_eq!(storage.get(&id2).unwrap().text, "second");
        }
        
        std::fs::remove_file(test_path).unwrap();
        std::fs::remove_file(test_index).unwrap();
        let _ = std::fs::remove_file(test_wal);
        let _ = std::fs::remove_file(test_vecindex);
        let _ = std::fs::remove_file(test_meta);
    }

    #[test]
    fn test_search() {
        let _ = std::fs::remove_file(".piramid/tests/test_search.db");
        let _ = std::fs::remove_file(".piramid/tests/test_search.db.index.db");
        let _ = std::fs::remove_file(".piramid/tests/test_search.db.wal.db");
        let _ = std::fs::remove_file(".piramid/tests/test_search.db.vecindex.db");
        
        let mut storage = Collection::open(".piramid/tests/test_search.db").unwrap();
        
        let vectors = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
            vec![0.9, 0.1, 0.0],
        ];
        
        for (i, vec) in vectors.iter().enumerate() {
            let entry = Document::new(vec.clone(), format!("vec{}", i));
            storage.insert(entry).unwrap();
        }
        
        let query = vec![1.0, 0.0, 0.0];
        let results = storage.search(&query, 2, Metric::Cosine, crate::search::SearchParams {
            mode: storage.config().execution,
            filter: None,
            filter_overfetch_override: None,
            search_config_override: None,
        });
        
        assert_eq!(results.len(), 2);
        
        drop(storage);
        std::fs::remove_file(".piramid/tests/test_search.db").unwrap();
        std::fs::remove_file(".piramid/tests/test_search.db.index.db").unwrap();
        let _ = std::fs::remove_file(".piramid/tests/test_search.db.wal.db");
        let _ = std::fs::remove_file(".piramid/tests/test_search.db.vecindex.db");
    }

    #[test]
    fn test_batch_search() {
        let _ = std::fs::remove_file(".piramid/tests/test_batch_search.db");
        let _ = std::fs::remove_file(".piramid/tests/test_batch_search.db.index.db");
        let _ = std::fs::remove_file(".piramid/tests/test_batch_search.db.wal.db");
        let _ = std::fs::remove_file(".piramid/tests/test_batch_search.db.vecindex.db");
        
        let mut storage = Collection::open(".piramid/tests/test_batch_search.db").unwrap();
        
        // Insert test vectors
        for i in 0..10 {
            let vec = vec![i as f32, 0.0, 0.0];
            let entry = Document::new(vec, format!("vec{}", i));
            storage.insert(entry).unwrap();
        }
        
        // Run multiple single searches to emulate batch
        let queries = vec![
            vec![0.0, 0.0, 0.0],
            vec![5.0, 0.0, 0.0],
            vec![9.0, 0.0, 0.0],
        ];
        for q in queries {
            let results = storage.search(&q, 2, Metric::Cosine, crate::search::SearchParams::default());
            assert_eq!(results.len(), 2);
        }
        
        drop(storage);
        std::fs::remove_file(".piramid/tests/test_batch_search.db").unwrap();
        std::fs::remove_file(".piramid/tests/test_batch_search.db.index.db").unwrap();
        let _ = std::fs::remove_file(".piramid/tests/test_batch_search.db.wal.db");
        let _ = std::fs::remove_file(".piramid/tests/test_batch_search.db.vecindex.db");
    }
}
