//! Tokenizers backed by the tokenizers crate, reading a checkpoint's tokenizer.json.

use std::path::Path;

use piramid_core::error::InferenceError;

use crate::inference::tokenizer::Tokenizer;

/// A tokenizer.json tokenizer.
#[derive(Debug)]
pub struct JsonTokenizer {
    inner: tokenizers::Tokenizer,
}

impl JsonTokenizer {
    /// Read tokenizer.json from the directory at path.
    pub fn load(path: &Path) -> Result<Self, InferenceError> {
        if !path.is_dir() {
            return Err(InferenceError::Load(format!(
                "{}: tokenizer path must be a directory holding tokenizer.json",
                path.display()
            )));
        }
        let file = path.join("tokenizer.json");
        let inner = tokenizers::Tokenizer::from_file(&file)
            .map_err(|e| InferenceError::Load(format!("{}: {e}", file.display())))?;
        Ok(Self { inner })
    }
}

impl Tokenizer for JsonTokenizer {
    fn encode(&self, text: &str) -> Result<Vec<u32>, InferenceError> {
        self.inner
            .encode(text, false)
            .map(|encoding| encoding.get_ids().to_vec())
            .map_err(|e| InferenceError::Runtime(format!("tokenize: {e}")))
    }

    fn encode_with_template(&self, text: &str) -> Result<Vec<u32>, InferenceError> {
        self.inner
            .encode(text, true)
            .map(|encoding| encoding.get_ids().to_vec())
            .map_err(|e| InferenceError::Runtime(format!("tokenize: {e}")))
    }

    fn decode(&self, tokens: &[u32], skip_special: bool) -> Result<String, InferenceError> {
        self.inner
            .decode(tokens, skip_special)
            .map_err(|e| InferenceError::Runtime(format!("detokenize: {e}")))
    }

    fn token_id(&self, token: &str) -> Option<u32> {
        self.inner.token_to_id(token)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "assertions in tests")]

    use super::*;

    #[test]
    fn a_tokenizer_path_that_is_a_file_is_refused() {
        let file = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
        let error = JsonTokenizer::load(file).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("must be a directory holding tokenizer.json"),
            "{error}"
        );
    }
}
