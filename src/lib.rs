// ## Crate organization

pub mod cli;
pub mod config;
pub mod embeddings;
pub mod error;
pub mod index;
pub mod metadata;
pub mod metrics;
pub mod quantization;
pub mod search;
pub mod server;
pub mod storage;
pub mod validation;

pub use config::*;
pub use embeddings::{EmbeddingConfig, EmbeddingError, EmbeddingProvider};
pub use error::{ErrorContext, PiramidError, Result};
pub use index::{
    FlatConfig, FlatIndex, HnswConfig, HnswIndex, IndexConfig, IndexStats, IndexType, IvfConfig,
    IvfIndex, VectorIndex,
};
pub use metadata::{metadata, Metadata, MetadataValue};
pub use metrics::Metric;
pub use quantization::QuantizedVector;
pub use search::query::{Filter, FilterCondition};
pub use search::{Hit, SearchParams};
pub use storage::{Collection, CollectionMetadata, Document};
