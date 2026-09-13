//! The resident metadata of every live document in a collection.

use std::collections::HashMap;

use piramid_core::metadata::{Metadata, MetadataValue};
use uuid::Uuid;

/// Per-document metadata for every live document, keyed by id, with no bound and no eviction.
#[derive(Default)]
pub struct MetadataStore {
    entries: HashMap<Uuid, Metadata>,
    /// Sum of the approximate bytes of every entry.
    usage_bytes: usize,
}

impl MetadataStore {
    /// An empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Every entry, keyed by id.
    pub fn entries(&self) -> &HashMap<Uuid, Metadata> {
        &self.entries
    }

    /// Insert or replace the metadata for id.
    pub fn put(&mut self, id: Uuid, metadata: Metadata) {
        self.usage_bytes += entry_usage_bytes(&metadata);
        if let Some(replaced) = self.entries.insert(id, metadata) {
            self.usage_bytes -= entry_usage_bytes(&replaced);
        }
    }

    /// Drop the entry for id.
    pub fn remove(&mut self, id: &Uuid) {
        if let Some(removed) = self.entries.remove(id) {
            self.usage_bytes -= entry_usage_bytes(&removed);
        }
    }

    /// Approximate resident bytes, kept as entries are put and removed.
    pub fn usage_bytes(&self) -> usize {
        self.usage_bytes
    }
}

/// Approximate resident bytes of one entry: its id, keys and values.
fn entry_usage_bytes(metadata: &Metadata) -> usize {
    std::mem::size_of::<Uuid>()
        + metadata
            .iter()
            .map(|(key, value)| key.capacity() + value_usage_bytes(value))
            .sum::<usize>()
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
