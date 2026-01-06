//! Piramid - Vector database for agentic applications
//!
//! Store embeddings, find similar ones. That's what vector databases do.
//!
//! ## Crate organization
//! - Core: storage, distance, metadata, query, search
//! - Server: HTTP API (axum-based, modular)
//! - Error handling: thiserror-based Result types

pub mod config;
pub mod distance;
pub mod error;
pub mod metadata;
pub mod query;
pub mod search;
pub mod server;
pub mod storage;

pub use config::Config;
pub use distance::DistanceMetric;
pub use error::{PiramidError, Result};
pub use metadata::{Metadata, MetadataValue, metadata};
pub use query::{Filter, FilterCondition};
pub use search::SearchResult;
pub use storage::{VectorEntry, VectorStorage};
