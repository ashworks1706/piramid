// Handler modules organized by functionality
pub mod collections;
pub mod config;
pub mod embeddings;
pub mod health;
pub mod ready;
pub mod vectors;
pub mod version;

// Re-export all handlers
pub use collections::*;
pub use config::*;
pub use embeddings::*;
pub use health::*;
pub use ready::*;
pub use vectors::*;
pub use version::*;
