// src/server/handlers/version.rs
// this file defines the handler for the /api/version endpoint, which returns the server version and optional git commit hash. It uses the CARGO_PKG_VERSION environment variable to get the version from Cargo.toml, and the GIT_COMMIT_HASH environment variable (if set) to include the git commit hash in the response. The response is returned as JSON with a simple struct that includes the version and optional git commit. This endpoint can be used by clients to check the version of the server they are interacting with, which can be helpful for debugging, compatibility checks, or displaying version information in client applications. By including the git commit hash, we can also provide more detailed information about the exact code version running on the server, which can be useful for tracking changes and diagnosing issues.

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
