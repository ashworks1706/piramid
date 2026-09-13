//! Use cases behind the HTTP handlers, and the API shapes they take and return.

pub mod admin;
pub mod api;
pub mod collection;
pub mod convert;
pub mod embedding;
pub mod generation;
pub mod vector;

/// Error message for a document id that is not in the collection.
pub const VECTOR_NOT_FOUND: &str = "Vector not found";
/// Error message for an embedding request when no provider is configured.
pub const EMBEDDING_NOT_CONFIGURED: &str = "Embedding service not configured";
