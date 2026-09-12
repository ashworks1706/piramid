//! What is held in memory, and what gives when the budget is reached.
//!
//! Vectors, metadata and embeddings are each configured separately.

use serde::{Deserialize, Serialize};

/// Every cache in the process.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct CacheConfig {
    /// Vectors held resident for search.
    pub vectors: VectorCacheConfig,
    /// Document metadata held for filter evaluation.
    pub metadata: MetadataCacheConfig,
    /// Embeddings kept to avoid repeat provider calls.
    pub embeddings: EmbeddingCacheConfig,

    /// Byte budget for resident vectors, shared across every loaded collection. None is
    /// unbounded.
    pub max_bytes: Option<u64>,
}

impl CacheConfig {
    /// A metadata cache of the given entry count, every other field default.
    pub fn with_size(size: usize) -> Self {
        CacheConfig {
            metadata: MetadataCacheConfig {
                entries: size,
                ..MetadataCacheConfig::default()
            },
            ..CacheConfig::default()
        }
    }

    /// Reject anything the build cannot honour.
    pub fn validate(&self) -> Result<(), String> {
        self.vectors.validate()?;
        self.metadata.validate()?;
        self.embeddings.validate()
    }
}

/// Vectors held resident for search.
///
/// An evicted vector cannot be scored until the store is rebuilt. Eviction is off by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct VectorCacheConfig {
    /// Entry ceiling. None keeps every vector of every loaded collection resident.
    pub entries: Option<usize>,

    /// Byte ceiling per collection. None is unbounded.
    pub max_bytes_per_collection: Option<u64>,

    /// What to drop when a bound is reached.
    pub eviction: EvictionPolicy,
}

impl Default for VectorCacheConfig {
    fn default() -> Self {
        VectorCacheConfig {
            entries: None,
            max_bytes_per_collection: None,
            eviction: EvictionPolicy::None,
        }
    }
}

impl VectorCacheConfig {
    fn validate(&self) -> Result<(), String> {
        if self.entries.is_some() || self.max_bytes_per_collection.is_some() {
            return Err(
                "runtime.cache.vectors: bounds are not enforced yet; the resident store is \
                 unbounded by design until the contiguous slab lands"
                    .into(),
            );
        }
        if self.eviction != EvictionPolicy::None {
            return Err("runtime.cache.vectors.eviction: not implemented yet".into());
        }
        Ok(())
    }
}

/// Document metadata held for filter evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct MetadataCacheConfig {
    /// Whether metadata is cached at all.
    pub enabled: bool,

    /// Entry ceiling.
    pub entries: usize,

    /// Entry lifetime in seconds. None never expires.
    pub ttl_seconds: Option<u64>,

    /// What to drop when the ceiling is reached.
    pub eviction: EvictionPolicy,
}

impl Default for MetadataCacheConfig {
    fn default() -> Self {
        MetadataCacheConfig {
            enabled: true,
            entries: 10_000,
            ttl_seconds: None,
            eviction: EvictionPolicy::Oldest,
        }
    }
}

impl MetadataCacheConfig {
    fn validate(&self) -> Result<(), String> {
        if self.ttl_seconds.is_some() {
            return Err("runtime.cache.metadata.ttl_seconds: not implemented yet".into());
        }
        if self.eviction != EvictionPolicy::Oldest {
            return Err("runtime.cache.metadata.eviction: only 'oldest' is implemented".into());
        }
        Ok(())
    }
}

/// Embeddings kept so identical text is not sent to a provider twice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct EmbeddingCacheConfig {
    /// Whether embeddings are cached at all.
    pub enabled: bool,

    /// Entry ceiling.
    pub entries: usize,
}

impl Default for EmbeddingCacheConfig {
    fn default() -> Self {
        EmbeddingCacheConfig {
            enabled: true,
            entries: 10_000,
        }
    }
}

impl EmbeddingCacheConfig {
    fn validate(&self) -> Result<(), String> {
        if self.enabled && self.entries == 0 {
            return Err(
                "runtime.cache.embeddings.entries: must be >= 1, or set enabled: false".into(),
            );
        }
        Ok(())
    }
}

/// What a cache drops first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum EvictionPolicy {
    /// Never evict.
    #[default]
    None,
    /// Oldest insertion first.
    Oldest,
    /// Least recently used.
    Lru,
}
