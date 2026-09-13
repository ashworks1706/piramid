//! The collection: a record store, resident state and a checkpoint policy in one queryable thing.

mod checkpoint;
mod compact;
pub(crate) mod limits;
mod manager;
mod open;
mod search_target;
mod state;

pub use checkpoint::CheckpointManager;
pub use compact::{compact, CompactStats};
pub use manager::{collection_names, record_path, CollectionHandle, CollectionManager};
pub use open::CollectionOpenOptions;
pub use state::Collection;
