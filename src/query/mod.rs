//! Query module - for building search queries with filters
//! 
//! Allows filtering vectors by metadata during similarity search.

mod filter;

pub use filter::{Filter, FilterCondition};
