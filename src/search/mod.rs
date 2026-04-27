// Search module - all search operations
// Future search types:
// - range_search: Find all vectors within a distance threshold
// - batch_search: Search multiple queries at once
// - hybrid_search: Combine vector + keyword search
// - recommendation_search: Find similar to these, not like those

mod types;
pub mod utils;
pub mod query;
pub mod engine;

pub use types::Hit;
pub use query::{Filter, FilterCondition};
pub use engine::{SearchParams, search_collection, search_batch_collection};
pub use crate::metrics::Metric;
