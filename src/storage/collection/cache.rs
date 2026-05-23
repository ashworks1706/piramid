use std::collections::{HashMap, VecDeque};
use uuid::Uuid;

use crate::config::CacheConfig;
use crate::index::VectorReader;
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
        self.metadata_order.retain(|cached_id| cached_id != &id);
        self.metadata.insert(id, metadata);
        self.metadata_order.push_back(id);
        self.enforce_item_limit();
    }

    pub fn remove(&mut self, id: &Uuid, remove_vector: bool) {
        if remove_vector {
            self.vectors.remove(id);
        }
        self.metadata.remove(id);
        self.metadata_order.retain(|cached_id| cached_id != id);
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
        self.metadata
            .iter()
            .map(|(id, metadata)| {
                std::mem::size_of_val(id)
                    + metadata
                        .iter()
                        .map(|(key, value)| key.capacity() + metadata_value_usage_bytes(value))
                        .sum::<usize>()
            })
            .sum()
    }

    pub fn vector_len(&self) -> usize {
        self.vectors.len()
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

impl VectorReader for CacheManager {
    fn get(&self, id: &Uuid) -> Option<&[f32]> {
        self.vectors.get(id).map(Vec::as_slice)
    }

    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = (Uuid, &'a [f32])> + 'a> {
        Box::new(
            self.vectors
                .iter()
                .map(|(id, vector)| (*id, vector.as_slice())),
        )
    }

    fn len(&self) -> usize {
        self.vectors.len()
    }
}

fn metadata_value_usage_bytes(value: &crate::metadata::MetadataValue) -> usize {
    match value {
        crate::metadata::MetadataValue::String(value) => value.capacity(),
        crate::metadata::MetadataValue::Integer(_)
        | crate::metadata::MetadataValue::Float(_)
        | crate::metadata::MetadataValue::Boolean(_)
        | crate::metadata::MetadataValue::Null => std::mem::size_of_val(value),
        crate::metadata::MetadataValue::Array(values) => {
            values.capacity() * std::mem::size_of::<crate::metadata::MetadataValue>()
                + values.iter().map(metadata_value_usage_bytes).sum::<usize>()
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
    }
}
