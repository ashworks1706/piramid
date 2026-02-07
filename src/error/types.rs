use thiserror::Error;
use std::io;
use axum::response::{IntoResponse, Response};
use axum::http::StatusCode;

pub type Result<T> = std::result::Result<T, PiramidError>;

#[derive(Error, Debug)]
pub enum PiramidError {
    // Storage errors
    #[error("Storage error: {0}")]
    Storage(#[from] super::storage::StorageError),

    // Index errors
    #[error("Index error: {0}")]
    Index(#[from] super::index::IndexError),

    // Server/API errors
    #[error("Server error: {0}")]
    Server(#[from] super::server::ServerError),

    // Embedding errors
    #[error("Embedding error: {0}")]
    Embedding(#[from] super::embedding::EmbeddingError),

    // IO errors
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    // Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),

    // JSON errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    // Generic errors
    #[error("{0}")]
    Other(String),
}

impl PiramidError {
    pub fn other<S: Into<String>>(msg: S) -> Self {
        Self::Other(msg.into())
    }

    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::Storage(e) => e.is_recoverable(),
            Self::Index(e) => e.is_recoverable(),
            Self::Server(e) => e.is_recoverable(),
            Self::Embedding(e) => e.is_recoverable(),
            Self::Io(_) => false,
            Self::Serialization(_) => false,
            Self::Json(_) => false,
            Self::Other(_) => false,
        }
    }

    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::Server(e) => e.status_code(),
            Self::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Index(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Embedding(_) => StatusCode::BAD_GATEWAY,
            Self::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Serialization(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Json(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Other(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for PiramidError {
    fn into_response(self) -> Response {
        match self {
            Self::Server(e) => e.into_response(),
            _ => {
                let status = self.status_code();
                let body = axum::Json(serde_json::json!({
                    "error": self.to_string(),
                    "code": status.as_u16(),
                }));
                (status, body).into_response()
            }
        }
    }
}
