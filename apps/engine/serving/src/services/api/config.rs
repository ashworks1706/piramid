//! Config status and reload responses.

use serde::Serialize;

/// The configuration in effect.
#[derive(Serialize)]
pub struct ConfigStatusResponse {
    /// The full configuration currently in effect.
    pub app_config: piramid_core::config::Config,
    /// Time of the last successful reload, or of startup when none has happened, in seconds since
    /// the Unix epoch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reloaded_at: Option<u64>,
}

/// Result of reloading configuration from disk and environment.
#[derive(Serialize)]
pub struct ConfigReloadResponse {
    /// True when the new configuration was applied.
    pub success: bool,
    /// Time of this reload, in seconds since the Unix epoch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reloaded_at: Option<u64>,
    /// The configuration now in effect.
    pub app_config: piramid_core::config::Config,
}
