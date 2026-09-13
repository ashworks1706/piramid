//! The embedding providers this build can construct.

mod factory;
pub mod ollama;
pub mod openai;
mod options;
#[cfg(feature = "inference-candle")]
pub mod piramid;

pub use factory::{create_embedder, EmbeddingProvider};
pub use ollama::OllamaEmbedder;
pub use openai::OpenAIEmbedder;
