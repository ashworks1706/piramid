//! Incremental detokenization of a growing token sequence.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]

use piramid_core::error::InferenceError;
use piramid_model::inference::tokenizer::{TextStream, Tokenizer};

/// Each token is one byte; bytes that do not form a character decode to the replacement.
struct Bytes;

impl Tokenizer for Bytes {
    fn encode(&self, text: &str) -> Result<Vec<u32>, InferenceError> {
        Ok(text.bytes().map(u32::from).collect())
    }

    fn encode_with_template(&self, text: &str) -> Result<Vec<u32>, InferenceError> {
        self.encode(text)
    }

    fn decode(&self, tokens: &[u32], _skip_special: bool) -> Result<String, InferenceError> {
        let bytes: Vec<u8> = tokens.iter().map(|&t| t as u8).collect();
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    fn token_id(&self, _token: &str) -> Option<u32> {
        None
    }
}

#[test]
fn a_multibyte_character_is_released_once_complete() {
    let mut stream = TextStream::new();
    let mut text = String::new();
    for token in Bytes.encode("a\u{00e9}b").unwrap() {
        let delta = stream.push(&Bytes, token).unwrap();
        assert!(!delta.contains('\u{FFFD}'));
        text.push_str(&delta);
    }
    assert_eq!(text, "a\u{00e9}b");
    assert_eq!(stream.tokens().len(), 4);
}
