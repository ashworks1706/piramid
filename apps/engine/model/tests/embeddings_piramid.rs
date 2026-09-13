//! The in-process embedding provider: pooling and refusals.
#![cfg(feature = "inference-candle")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]

use piramid_core::error::embedding::EmbeddingError;
use piramid_core::error::InferenceError;
use piramid_model::embeddings::providers::piramid::{
    embed_tokens, model_error, normalize, token_count,
};
use piramid_model::inference::architecture::Architecture;
use piramid_model::inference::backends::candle::qwen::testing::tiny_model;

#[test]
fn a_zero_or_non_finite_norm_is_an_error() {
    assert!(normalize(&mut [0.0, 0.0]).is_err());
    assert!(normalize(&mut [f32::NAN, 1.0]).is_err());
    assert!(normalize(&mut [f32::INFINITY, 1.0]).is_err());
    let mut vector = [3.0, 4.0];
    normalize(&mut vector).unwrap();
    assert_eq!(vector, [0.6, 0.8]);
}

#[test]
fn refused_text_is_invalid_input_and_not_retried() {
    for error in [
        token_count(0, 8).unwrap_err(),
        token_count(9, 8).unwrap_err(),
        model_error(InferenceError::InvalidRequest("too long".to_string())),
    ] {
        assert!(matches!(error, EmbeddingError::InvalidInput(_)), "{error}");
        assert!(!error.is_recoverable(), "{error}");
    }
    assert_eq!(token_count(8, 8).unwrap(), 8);
    assert!(matches!(
        model_error(InferenceError::Runtime("norm NaN".to_string())),
        EmbeddingError::InvalidResponse(_)
    ));
}

#[test]
fn an_embedding_is_unit_length_and_depends_on_the_text() {
    let mut model = tiny_model(Architecture::Qwen3, 9, 32);
    let a = embed_tokens(&mut model, vec![1, 2, 3]).unwrap();
    let b = embed_tokens(&mut model, vec![1, 2, 4]).unwrap();
    let again = embed_tokens(&mut model, vec![1, 2, 3]).unwrap();
    let norm: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!((norm - 1.0).abs() < 1e-4);
    assert_eq!(a.len(), 64);
    assert_ne!(a, b);
    for (x, y) in a.iter().zip(&again) {
        assert!((x - y).abs() < 1e-5);
    }
}
