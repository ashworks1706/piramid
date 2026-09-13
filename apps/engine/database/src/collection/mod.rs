//! The collection: the object that composes a record store, its resident state and a checkpoint
//! policy into one queryable thing.
//!
//! This module decides lifecycle: when a collection opens, when it checkpoints, when it compacts,
//! and what a write is allowed to do.
//!
//! [state] holds what a collection owns, and every other file here is one operation on that state.
//! CollectionHandle is the shared pointer callers hold, and Collection is the state behind its
//! lock. [search_target] turns a collection into a SearchTarget.

mod checkpoint;
mod compact;
pub(crate) mod limits;
mod manager;
mod open;
mod search_target;
mod state;

pub use checkpoint::CheckpointManager;
pub use compact::{compact, CompactStats};
pub use manager::{CollectionHandle, CollectionManager};
pub use open::CollectionOpenOptions;
pub use state::Collection;
