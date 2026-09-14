//! Umbrella crate: re-exports every workspace crate under one namespace.

pub mod animation;
pub mod console;
pub mod support;

pub use piramid_core::{config, document, error, metadata, observability, stats, validation};

pub use piramid_database::{resident, search, storage};
// Items the database crate exposes only at its root.
pub use piramid_database::{
    collection_names, compact, record_path, CheckpointManager, Collection, CollectionHandle,
    CollectionManager, CollectionOpenOptions, CompactStats,
};
pub use piramid_hardware::{compute, gpu};
pub use piramid_model::{embeddings, fusion, inference};
pub use piramid_serving::{disk, http, services, state};
