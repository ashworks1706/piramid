//! Store embeddings, find similar ones. That's what vector databases do.
//!
//! ## Crate organization
//! - Core: storage, metrics, metadata, query, search
//! - Server: HTTP API (axum-based, modular)
//! - Error handling: thiserror-based Result types

pub mod config;
pub mod metrics;
pub mod error;
pub mod metadata;
pub mod query;
pub mod search;
pub mod server;
pub mod storage;
pub mod embeddings;

pub use config::Config;
pub use metrics::SimilarityMetric;
pub use error::{PiramidError, Result};
pub use metadata::{Metadata, MetadataValue, metadata};
pub use query::{Filter, FilterCondition};
pub use search::SearchResult;
pub use storage::{VectorEntry, VectorStorage};
pub use embeddings::{EmbeddingConfig, EmbeddingProvider, EmbeddingError};
