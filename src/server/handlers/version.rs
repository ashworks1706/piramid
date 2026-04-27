// src/server/handlers/version.rs
// this file defines the handler for the /api/version endpoint, which returns the server version and optional git commit hash

use axum::response::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct VersionResponse {
    pub version: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_commit: Option<&'static str>,
}

// GET /api/version - returns binary version and optional git hash
pub async fn version() -> Json<VersionResponse> {
    Json(VersionResponse {
        version: env!("CARGO_PKG_VERSION"),
        git_commit: option_env!("GIT_COMMIT_HASH"),
    })
}
