//! The bounded metadata cache: oldest-first eviction, safe to drop entirely.

use std::collections::{HashMap, VecDeque};

use piramid_core::config::CacheConfig;
use piramid_core::metadata::{Metadata, MetadataValue};
use uuid::Uuid;

/// Cached per-document metadata, bounded by CacheConfig::max_size entries.
///
/// A miss re-reads the record store.
pub struct MetadataCache {
    config: CacheConfig,
    entries: HashMap<Uuid, Metadata>,
    order: VecDeque<Uuid>,
}

impl MetadataCache {
    /// An empty cache bounded by config.
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            entries: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    /// The cached entries, keyed by id.
    pub fn entries(&self) -> &HashMap<Uuid, Metadata> {
        &self.entries
    }

    /// Cache metadata for id, evicting oldest entries past the bound.
    pub fn put(&mut self, id: Uuid, metadata: Metadata) {
        if !self.config.metadata.enabled {
            return;
        }
        self.order.retain(|cached_id| cached_id != &id);
        self.entries.insert(id, metadata);
        self.order.push_back(id);
        self.enforce_item_limit();
    }

    /// Drop the entry for id.
    pub fn remove(&mut self, id: &Uuid) {
        self.entries.remove(id);
        self.order.retain(|cached_id| cached_id != id);
    }

    /// Drop every entry, returning the bytes freed.
    pub fn clear(&mut self) -> usize {
        let freed = self.usage_bytes();
        self.entries.clear();
        self.order.clear();
        freed
    }

    /// Approximate resident bytes.
    pub fn usage_bytes(&self) -> usize {
        self.entries
            .iter()
            .map(|(id, metadata)| {
                std::mem::size_of_val(id)
                    + metadata
                        .iter()
                        .map(|(key, value)| key.capacity() + value_usage_bytes(value))
                        .sum::<usize>()
            })
            .sum()
    }

    fn enforce_item_limit(&mut self) {
        if self.config.metadata.entries == 0 {
            self.entries.clear();
            self.order.clear();
            return;
        }

        while self.entries.len() > self.config.metadata.entries {
            match self.order.pop_front() {
                Some(id) => {
                    self.entries.remove(&id);
                }
                None => break,
            }
        }
    }
}

fn value_usage_bytes(value: &MetadataValue) -> usize {
    match value {
        MetadataValue::String(value) => value.capacity(),
        MetadataValue::Integer(_)
        | MetadataValue::Float(_)
        | MetadataValue::Boolean(_)
        | MetadataValue::Null => std::mem::size_of_val(value),
        MetadataValue::Array(values) => {
            values.capacity() * std::mem::size_of::<MetadataValue>()
                + values.iter().map(value_usage_bytes).sum::<usize>()
        }
    }
}
