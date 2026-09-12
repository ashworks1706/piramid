//! Provider options from configuration, as JSON request fields.

use crate::embeddings::embedder::EmbeddingResult;
use piramid_core::error::embedding::EmbeddingError;

/// The configured options as request fields: an object, or nothing for null.
pub(super) fn request_options(
    options: &serde_json::Value,
) -> EmbeddingResult<serde_json::Map<String, serde_json::Value>> {
    match options {
        serde_json::Value::Null => Ok(serde_json::Map::new()),
        serde_json::Value::Object(fields) => Ok(fields.clone()),
        _ => Err(EmbeddingError::ConfigError(
            "startup.embedding.options: must be null or a mapping".to_string(),
        )),
    }
}
