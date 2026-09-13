//! The request and response shapes of the HTTP API, one file per handler module.

mod collections;
mod config;
mod embeddings;
mod generation;
mod health;
mod metrics;
mod search;
mod vectors;
mod version;

pub use collections::*;
pub use config::*;
pub use embeddings::*;
pub use generation::*;
pub use health::*;
pub use metrics::*;
pub use search::*;
pub use vectors::*;
pub use version::*;
