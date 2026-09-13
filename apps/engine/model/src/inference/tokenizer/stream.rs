//! Incremental detokenization: text is released only once the tokens behind it decode to
//! complete characters.

use piramid_core::error::InferenceError;

use crate::inference::tokenizer::Tokenizer;

/// Turns a growing token sequence into text deltas.
#[derive(Debug, Default, Clone)]
pub struct TextStream {
    tokens: Vec<u32>,
    prefix_offset: usize,
    read_offset: usize,
}

impl TextStream {
    /// An empty stream.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add one token and return the text it completes, which may be empty.
    pub fn push(
        &mut self,
        tokenizer: &dyn Tokenizer,
        token: u32,
    ) -> Result<String, InferenceError> {
        self.tokens.push(token);
        let prefix = tokenizer.decode(&self.tokens[self.prefix_offset..self.read_offset], true)?;
        let mut full = tokenizer.decode(&self.tokens[self.prefix_offset..], true)?;
        if full.len() > prefix.len()
            && !full.ends_with('\u{FFFD}')
            && full.is_char_boundary(prefix.len())
        {
            full.replace_range(..prefix.len(), "");
            self.prefix_offset = self.read_offset;
            self.read_offset = self.tokens.len();
            Ok(full)
        } else {
            Ok(String::new())
        }
    }

    /// Every token pushed so far.
    pub fn tokens(&self) -> &[u32] {
        &self.tokens
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "assertions in tests")]

    use super::*;

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
}
