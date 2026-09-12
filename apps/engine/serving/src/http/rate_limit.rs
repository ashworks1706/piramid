//! Per-client token bucket rate limiting, keyed by peer IP address.

use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::header::RETRY_AFTER;
use axum::http::HeaderValue;
use axum::response::{IntoResponse, Response};
use governor::middleware::NoOpMiddleware;
use piramid_core::config::RateLimitConfig;
use piramid_core::error::ServerError;
use tower_governor::governor::{GovernorConfig, GovernorConfigBuilder};
use tower_governor::key_extractor::PeerIpKeyExtractor;
use tower_governor::{GovernorError, GovernorLayer};

use super::ApiError;

type Limiter = GovernorConfig<PeerIpKeyExtractor, NoOpMiddleware>;

/// One bucket per client address, shared by every route it is layered on.
#[derive(Clone)]
pub struct RateLimit {
    limiter: Arc<Limiter>,
}

impl RateLimit {
    /// A limiter for the configured rate and burst. A zero rate or burst is an error.
    pub fn new(config: &RateLimitConfig) -> Result<Self, String> {
        config.validate()?;
        let period_nanos = (1_000_000_000 / u64::from(config.requests_per_second)).max(1);
        let limiter = GovernorConfigBuilder::default()
            .period(Duration::from_nanos(period_nanos))
            .burst_size(config.burst)
            .finish()
            .ok_or_else(|| "startup.http.rate_limit: rate and burst must be > 0".to_string())?;
        Ok(Self {
            limiter: Arc::new(limiter),
        })
    }

    /// The tower layer. Requests without peer connection info are answered with 500.
    pub fn layer(&self) -> GovernorLayer<PeerIpKeyExtractor, NoOpMiddleware, Body> {
        GovernorLayer::new(self.limiter.clone()).error_handler(rejection)
    }

    /// Drops the buckets of clients that are back at full capacity, on the given period, forever.
    pub async fn sweep(self, period: Duration) {
        let mut interval = tokio::time::interval(period);
        loop {
            interval.tick().await;
            self.limiter.limiter().retain_recent();
        }
    }
}

/// The response for a request the limiter refused.
fn rejection(error: GovernorError) -> Response {
    match error {
        GovernorError::TooManyRequests { wait_time, .. } => {
            let mut response = ApiError::from(ServerError::RateLimitExceeded).into_response();
            // The header is the whole seconds of the wait plus one.
            response
                .headers_mut()
                .insert(RETRY_AFTER, HeaderValue::from(wait_time.saturating_add(1)));
            response
        }
        other => {
            ApiError::from(ServerError::Internal(format!("rate limit: {other}"))).into_response()
        }
    }
}
