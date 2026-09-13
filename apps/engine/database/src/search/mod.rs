//! Query execution: exact scoring, filtering and ranking.

pub mod engine;

pub use engine::{search, search_batch, SearchParams, SearchTarget};
