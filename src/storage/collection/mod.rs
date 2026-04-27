mod storage;
mod operations;
mod builder;
mod cache;
mod persistence;
mod search;
mod dup;
mod compact;

pub use storage::Collection;
pub use builder::CollectionBuilder;
pub use compact::{compact, CompactStats};
pub use dup::{find_duplicates, DuplicateHit};

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
//  allows users to directly pass a CollectionConfig when opening a collection, and it will be automatically converted into the appropriate open options
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
        search::search(self, query, k, metric, params)
    }

    pub fn search_batch(&self, queries: &[Vec<f32>], k: usize, metric: Metric) -> Vec<Vec<Hit>> {
        search::search_batch(self, queries, k, metric)
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
