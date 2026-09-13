//! Endpoint handlers, one module per resource.
pub mod collections;
pub mod config;
pub mod embeddings;
pub mod generation;
pub mod health;
pub mod ready;
pub mod vectors;
pub mod version;
pub use collections::*;
pub use config::*;
pub use embeddings::*;
pub use generation::*;
pub use health::*;
pub use ready::*;
pub use vectors::*;
pub use version::*;
