pub mod registry;

pub use crate::storage::collection::{
    Collection, CollectionBuilder, CollectionOpenOptions, CompactStats, DuplicateHit,
};
pub use registry::{CollectionHandle, CollectionRegistry};
