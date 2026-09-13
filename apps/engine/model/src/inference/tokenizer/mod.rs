//! Text to tokens and back: the [Tokenizer] contract a backend implements, the checkpoint's chat
//! template, and incremental decoding for streamed output.

pub mod chat;
pub mod stream;

use piramid_core::error::InferenceError;

pub use chat::{ChatMessage, ChatTemplate};
pub use stream::TextStream;

/// Converts between text and the token ids of one model.
pub trait Tokenizer: Send + Sync {
    /// Token ids of text. Special tokens written in the text are recognised as special tokens.
    fn encode(&self, text: &str) -> Result<Vec<u32>, InferenceError>;

    /// Text of token ids, with special tokens left out when skip_special is set.
    fn decode(&self, tokens: &[u32], skip_special: bool) -> Result<String, InferenceError>;

    /// The id of a token written exactly as the vocabulary holds it.
    fn token_id(&self, token: &str) -> Option<u32>;
}
