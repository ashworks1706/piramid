//! Request bodies and status errors of the OpenAI wire format.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]

use piramid_core::config::EmbeddingConfig;
use piramid_core::error::embedding::EmbeddingError;
use piramid_model::embeddings::providers::openai::{status_error, OpenAIEmbedder};

#[test]
fn options_are_merged_into_the_request_body() {
    let config: EmbeddingConfig = serde_json::from_value(serde_json::json!({
        "provider": "openai",
        "model": "text-embedding-3-small",
        "options": { "dimensions": 256, "user": "docs" }
    }))
    .unwrap();
    let body = OpenAIEmbedder::new(&config).unwrap().request_body("hello");
    assert_eq!(body["dimensions"], 256);
    assert_eq!(body["user"], "docs");
    assert_eq!(body["model"], "text-embedding-3-small");
    assert_eq!(body["input"], "hello");
}

#[test]
fn refused_input_and_unknown_models_are_not_retried() {
    let error = |code: u16| status_error(reqwest::StatusCode::from_u16(code).unwrap(), "no".into());
    for code in [400, 422] {
        assert!(
            matches!(error(code), EmbeddingError::InvalidInput(_)),
            "{code}"
        );
        assert!(!error(code).is_recoverable(), "{code}");
    }
    assert!(matches!(error(404), EmbeddingError::InvalidModel(_)));
    assert!(!error(404).is_recoverable());
    assert!(matches!(
        error(401),
        EmbeddingError::AuthenticationFailed(_)
    ));
    assert!(matches!(error(429), EmbeddingError::RateLimitExceeded));
    assert!(matches!(error(503), EmbeddingError::ApiError(_)));
    assert!(error(503).is_recoverable());
}
