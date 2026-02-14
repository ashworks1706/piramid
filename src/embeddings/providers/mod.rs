mod factory;
pub mod openai;
pub mod ollama;
pub mod local;

pub use factory::{EmbeddingProvider, create_embedder};
pub use openai::OpenAIEmbedder;
pub use ollama::OllamaEmbedder;
pub use local::LocalEmbedder;
