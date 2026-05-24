// Storage module - handles records, write-ahead logging, metadata sidecars, and mmap persistence.

pub mod collection {
    pub use crate::collections::{
        compact, find_duplicates, Collection, CollectionBuilder, CollectionOpenOptions,
        CompactStats, DuplicateHit,
    };
}

pub mod document;
pub mod metadata;
pub mod persistence;
pub mod record_store;
pub mod wal;
pub use crate::collections::Collection;
pub use document::Document;
pub use metadata::CollectionMetadata;
