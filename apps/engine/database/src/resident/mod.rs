//! The resident state of one collection, behind one entry point: [ResidentManager].

mod manager;
mod metadata_store;
mod vector_store;

pub use manager::ResidentManager;
pub use metadata_store::MetadataStore;
pub use vector_store::{ordinal_for_row, VectorStore};
