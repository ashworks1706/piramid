// ## Crate organization
// - Core: storage, metrics, metadata, query, search
// - Server: HTTP API (axum-based, modular)
// - Error handling: thiserror-based Result types

pub mod config;
pub mod metrics;
pub mod error;
pub mod metadata;
pub mod query;
pub mod search;
pub mod server;
pub mod storage;
pub mod embeddings;
pub mod index;
pub mod quantization;

pub use config::Config;
pub use metrics::Metric;
pub use error::{PiramidError, Result, ErrorContext};
pub use metadata::{Metadata, MetadataValue, metadata};
pub use query::{Filter, FilterCondition};
pub use search::Hit;
pub use storage::{Document, Collection};
pub use embeddings::{EmbeddingConfig, EmbeddingProvider, EmbeddingError};
pub use index::{
    HnswIndex, HnswConfig, 
    FlatIndex, FlatConfig,
    IvfIndex, IvfConfig,
    VectorIndex, IndexConfig, IndexType, IndexStats,
};
pub use quantization::QuantizedVector;
