// Handler modules organized by functionality
pub mod health;
pub mod collections;
pub mod vectors;
pub mod embeddings;
pub mod config;
pub mod ready;
pub mod version;

// Re-export all handlers
pub use health::*;
pub use collections::*;
pub use vectors::*;
pub use embeddings::*;
pub use config::*;
pub use ready::*;
pub use version::*;
