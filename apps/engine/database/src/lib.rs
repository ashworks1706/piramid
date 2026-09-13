//! The database: where documents live, how they are found, and the object that owns both.
//!
//! [storage] holds records, the write-ahead log, mmap and sidecars. [resident] holds the resident
//! vectors and metadata of a collection. [search] scores a query against every stored vector and
//! ranks the hits. The collection composes a record store, its resident state and a checkpoint
//! policy into one queryable thing.

pub mod resident;
pub mod search;
pub mod storage;

mod collection;
mod document;

pub use collection::{
    compact, CheckpointManager, Collection, CollectionHandle, CollectionManager,
    CollectionOpenOptions, CompactStats,
};
pub use resident::ResidentManager;
