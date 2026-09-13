//! In-memory state for one collection, behind one entry point: [CacheManager].
//!
//! [VectorStore] is resident working state. The ANN indexes resolve candidate ids through it, and
//! evicting an entry breaks search. [MetadataCache] is bounded, evicted oldest-first, and safe to
//! drop.

mod manager;
mod metadata_cache;
mod vector_store;

pub use manager::CacheManager;
pub use metadata_cache::MetadataCache;
pub use vector_store::{ordinal_for_row, VectorStore};
