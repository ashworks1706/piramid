use std::collections::{HashMap, VecDeque};
use uuid::Uuid;

use crate::config::CacheConfig;
use crate::metadata::Metadata;
use crate::storage::collection::operations;
use crate::storage::collection::storage::Collection;

pub struct CacheManager {
    config: CacheConfig,
    vectors: HashMap<Uuid, Vec<f32>>,
    metadata: HashMap<Uuid, Metadata>,
    metadata_order: VecDeque<Uuid>,
}

impl CacheManager {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            vectors: HashMap::new(),
            metadata: HashMap::new(),
            metadata_order: VecDeque::new(),
        }
    }

    pub fn vectors(&self) -> &HashMap<Uuid, Vec<f32>> {
        &self.vectors
    }

    pub fn metadata(&self) -> &HashMap<Uuid, Metadata> {
        &self.metadata
    }

    pub fn put_vector(&mut self, id: Uuid, vector: Vec<f32>) {
        self.vectors.insert(id, vector);
    }

    pub fn put_metadata(&mut self, id: Uuid, metadata: Metadata) {
        if !self.config.enabled {
            return;
        }
        self.metadata.insert(id, metadata);
        self.metadata_order.push_back(id);
        self.enforce_item_limit();
    }

    pub fn remove(&mut self, id: &Uuid, remove_vector: bool) {
        if remove_vector {
            self.vectors.remove(id);
        }
        self.metadata.remove(id);
    }

    pub fn clear_all(&mut self) {
        self.vectors.clear();
        self.metadata.clear();
        self.metadata_order.clear();
    }

    pub fn clear_metadata(&mut self) -> usize {
        let freed = self.metadata_usage_bytes();
        self.metadata.clear();
        self.metadata_order.clear();
        freed
    }

    pub fn memory_usage_bytes(&self) -> usize {
        self.vector_usage_bytes() + self.metadata_usage_bytes()
    }

    pub fn metadata_usage_bytes(&self) -> usize {
        self.metadata.len() * std::mem::size_of::<(Uuid, Metadata)>()
    }

    pub fn vector_len(&self) -> usize {
        self.vectors.len()
    }

    pub fn metadata_contains(&self, id: &Uuid) -> bool {
        self.metadata.contains_key(id)
    }

    pub fn vector_contains(&self, id: &Uuid) -> bool {
        self.vectors.contains_key(id)
    }

    fn vector_usage_bytes(&self) -> usize {
        self.vectors
            .values()
            .map(|vec| std::mem::size_of::<Uuid>() + vec.len() * std::mem::size_of::<f32>())
            .sum()
    }

    fn enforce_item_limit(&mut self) {
        if self.config.max_size == 0 {
            self.metadata.clear();
            self.metadata_order.clear();
            return;
        }

        while self.metadata.len() > self.config.max_size {
            match self.metadata_order.pop_front() {
                Some(id) => {
                    self.metadata.remove(&id);
                }
                None => break,
            }
        }
    }
}

pub fn rebuild(collection: &mut Collection) {
    let mut cache = CacheManager::new(collection.config.cache);
    for id in collection.index.keys() {
        if let Some(entry) = operations::get(collection, id) {
            cache.put_vector(*id, entry.get_vector());
            cache.put_metadata(*id, entry.metadata.clone());
        }
    }
    collection.cache = cache;
}

pub fn ensure_consistent(collection: &mut Collection) {
    if collection.cache.vector_len() != collection.index.len() {
        rebuild(collection);
        return;
    }

    for id in collection.index.keys() {
        if !collection.cache.vector_contains(id) {
            rebuild(collection);
            break;
        }
        if collection.config.cache.enabled && !collection.cache.metadata_contains(id) {
            rebuild(collection);
            break;
        }
    }
}
