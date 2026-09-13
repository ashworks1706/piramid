//! The database: where vectors live, how they are found, and the object that owns both.
//!
//! [storage] holds records, the write-ahead log, mmap and sidecars. [index] and [search] hold
//! traversal structures and the planning, filtering, scoring and ranking over them. The collection
//! composes a record store, its caches, a checkpoint policy and an index into one queryable thing.

pub mod cache;
pub mod index;
pub mod search;
pub mod storage;

mod collection;
mod document;

pub use cache::{CacheManager, MetadataCache, VectorStore};
pub use collection::{
    compact, find_duplicates, CheckpointManager, Collection, CollectionHandle, CollectionManager,
    CollectionOpenOptions, CompactStats,
};
pub use search::DuplicatePair;
