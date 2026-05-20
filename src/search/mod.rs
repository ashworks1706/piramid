// Search module - all search operations
// Future search types:
// - range_search: Find all vectors within a distance threshold
// - batch_search: Search multiple queries at once
// - hybrid_search: Combine vector + keyword search
// - recommendation_search: Find similar to these, not like those

pub mod engine;
pub mod query;
mod types;
pub mod utils;

pub use crate::metrics::Metric;
pub use engine::{search_batch_collection, search_collection, SearchParams};
pub use query::{Filter, FilterCondition};
pub use types::Hit;
