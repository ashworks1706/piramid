// Storage module - handles vector persistence with memory-mapped files

pub mod collection;
mod document;
mod metadata;
mod persistence;
pub mod wal;
pub use collection::Collection;
pub use document::Document;
pub use metadata::CollectionMetadata;
