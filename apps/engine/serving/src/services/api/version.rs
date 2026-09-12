//! Build identity.

use serde::Serialize;

/// Binary version, plus the git hash of the build when one was baked in.
#[derive(Serialize)]
pub struct VersionResponse {
    /// Version of the running binary.
    pub version: &'static str,
    /// Commit hash of the build. Absent when none was set at compile time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_commit: Option<&'static str>,
}
