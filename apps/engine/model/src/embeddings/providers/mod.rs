mod factory;
pub mod ollama;
pub mod openai;
mod options;

pub use factory::{create_embedder, EmbeddingProvider};
pub use ollama::OllamaEmbedder;
pub use openai::OpenAIEmbedder;
