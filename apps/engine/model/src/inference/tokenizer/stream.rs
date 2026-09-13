//! Incremental detokenization: text is released only once its tokens decode to full characters.

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
