//! Persistence primitives: records, WAL, sidecars, mmap, and vector layout.

pub mod codec;
pub mod manifest;
pub mod record_store;
pub mod sidecars;
pub mod vectors;
pub mod wal;

pub use manifest::CollectionMetadata;
pub use sidecars::SidecarManager;
pub use vectors::{HashMapVectorReader, VectorReader, VectorSlab};
