//! The database: where documents live, how they are found, and the object that owns both.

pub mod resident;
pub mod search;
pub mod storage;

mod collection;
mod document;

pub use collection::{
    collection_names, compact, record_path, CheckpointManager, Collection, CollectionHandle,
    CollectionManager, CollectionOpenOptions, CompactStats,
};
pub use resident::ResidentManager;
