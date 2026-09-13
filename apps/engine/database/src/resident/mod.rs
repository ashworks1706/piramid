//! The resident state of one collection, behind one entry point: [ResidentManager].
//!
//! [VectorStore] holds every live vector and [MetadataStore] holds the metadata of every live
//! document. Both are resident for as long as the collection is open, are never evicted, and are
//! rebuilt from the record store at open.

mod manager;
mod metadata_store;
mod vector_store;

pub use manager::ResidentManager;
pub use metadata_store::MetadataStore;
pub use vector_store::{ordinal_for_row, VectorStore};
