//! Umbrella crate: re-exports every workspace crate under one namespace. Crate boundaries and the
//! dependency rule are in docs/ARCHITECTURE.md. Also holds the console and the support
//! bundle that the binary drives.

pub mod console;
pub mod support;

pub use piramid_core::{config, document, error, metadata, observability, stats, validation};

pub use piramid_database::{cache, index, search, storage};
// Items the database crate exposes only at its root.
pub use piramid_database::{
    compact, find_duplicates, CheckpointManager, Collection, CollectionHandle, CollectionManager,
    CollectionOpenOptions, CompactStats,
};
pub use piramid_hardware::{compute, gpu};
pub use piramid_model::{embeddings, fusion, inference};
pub use piramid_serving::{cluster, disk, http, services, state};
