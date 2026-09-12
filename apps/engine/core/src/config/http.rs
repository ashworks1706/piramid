//! How the HTTP server admits, limits and releases requests.

use serde::{Deserialize, Serialize};

/// Name of the environment variable holding the server API key.
pub const API_KEY_ENV: &str = "PIRAMID_API_KEY";

/// Authentication, rate limiting and shutdown for the HTTP server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct HttpConfig {
    /// Who may call the server.
    pub auth: AuthConfig,

    /// Per-client token bucket. None serves every request without a limit.
    pub rate_limit: Option<RateLimitConfig>,

    /// Seconds in-flight requests get to finish after a shutdown signal.
    pub drain_timeout_secs: u64,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            auth: AuthConfig::default(),
            rate_limit: Some(RateLimitConfig::default()),
            drain_timeout_secs: 30,
        }
    }
}

impl HttpConfig {
    /// Refuses settings that contradict each other or cannot be served.
    pub fn validate(&self) -> Result<(), String> {
        self.auth.validate()?;
        if let Some(rate_limit) = &self.rate_limit {
            rate_limit.validate()?;
        }
        Ok(())
    }
}

/// Bearer key authentication.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct AuthConfig {
    /// Serve a non-loopback address with no API key.
    pub allow_unauthenticated: bool,

    /// The key every protected route requires. Set from PIRAMID_API_KEY only.
    #[serde(skip)]
    pub api_key: Option<ApiKey>,
}

impl AuthConfig {
    /// Refuses settings that contradict each other or cannot be served.
    pub fn validate(&self) -> Result<(), String> {
        if self.allow_unauthenticated && self.api_key.is_some() {
            return Err(format!(
                "startup.http.auth.allow_unauthenticated is true while {API_KEY_ENV} is set; \
                 unset one of them"
            ));
        }
        Ok(())
    }
}

/// A secret bearer key. Its Debug form is redacted.
#[derive(Clone, PartialEq, Eq)]
pub struct ApiKey(String);

impl ApiKey {
    /// A key from its text. An empty key is an error.
    pub fn new(key: String) -> Result<Self, String> {
        if key.is_empty() {
            return Err("the key is empty".to_string());
        }
        Ok(Self(key))
    }

    /// The key text.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ApiKey(<redacted>)")
    }
}

/// A token bucket kept per client address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct RateLimitConfig {
    /// Tokens added to each client's bucket per second.
    pub requests_per_second: u32,

    /// Bucket capacity: requests a client may make at once before the rate applies.
    pub burst: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 100,
            burst: 200,
        }
    }
}

impl RateLimitConfig {
    /// Refuses settings that contradict each other or cannot be served.
    pub fn validate(&self) -> Result<(), String> {
        if self.requests_per_second == 0 {
            return Err(
                "startup.http.rate_limit.requests_per_second: must be > 0, or set rate_limit to null"
                    .into(),
            );
        }
        if self.burst == 0 {
            return Err(
                "startup.http.rate_limit.burst: must be > 0, or set rate_limit to null".into(),
            );
        }
        Ok(())
    }
}
