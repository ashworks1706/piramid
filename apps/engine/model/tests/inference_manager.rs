//! The end-of-sequence token set a loaded model stops on.
#![cfg(feature = "inference-candle")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]

use piramid_core::error::InferenceError;
use piramid_model::inference::manager::eos_set;
use piramid_model::inference::tokenizer::Tokenizer;

struct Vocabulary;

impl Tokenizer for Vocabulary {
    fn encode(&self, _text: &str) -> Result<Vec<u32>, InferenceError> {
        Ok(Vec::new())
    }

    fn encode_with_template(&self, _text: &str) -> Result<Vec<u32>, InferenceError> {
        Ok(Vec::new())
    }

    fn decode(&self, _tokens: &[u32], _skip_special: bool) -> Result<String, InferenceError> {
        Ok(String::new())
    }

    fn token_id(&self, token: &str) -> Option<u32> {
        (token == "<|im_end|>").then_some(9)
    }
}

#[test]
fn the_eos_set_joins_the_checkpoint_ids_and_the_template_token() {
    let ids = eos_set(&[2], Some("<|im_end|>"), &Vocabulary).unwrap();
    assert_eq!(ids, [2, 9].into_iter().collect());
    let ids = eos_set(&[2], None, &Vocabulary).unwrap();
    assert_eq!(ids, [2].into_iter().collect());
}

#[test]
fn an_eos_token_outside_the_vocabulary_is_a_load_error() {
    let error = eos_set(&[2], Some("</s>"), &Vocabulary).unwrap_err();
    assert!(matches!(error, InferenceError::Load(_)), "{error}");
    assert!(error.to_string().contains("</s>"), "{error}");
}

#[test]
fn a_checkpoint_with_no_end_of_sequence_token_is_a_load_error() {
    let error = eos_set(&[], None, &Vocabulary).unwrap_err();
    assert!(
        error.to_string().contains("no end-of-sequence token"),
        "{error}"
    );
}
