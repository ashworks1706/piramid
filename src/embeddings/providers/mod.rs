mod factory;
pub mod local;
pub mod ollama;
pub mod openai;

pub use factory::{create_embedder, EmbeddingProvider};
pub use local::LocalEmbedder;
pub use ollama::OllamaEmbedder;
pub use openai::OpenAIEmbedder;
