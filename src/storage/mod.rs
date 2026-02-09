// Storage module - handles vector persistence with memory-mapped files

mod document;
mod collection;
mod metadata;
mod persistence;
pub mod wal;

pub use document::Document;
pub use collection::Collection;
pub use metadata::CollectionMetadata;
