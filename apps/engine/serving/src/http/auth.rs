//! Bearer key authentication for protected routes.

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::header::{AUTHORIZATION, WWW_AUTHENTICATE};
use axum::http::HeaderValue;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use piramid_core::config::ApiKey;
use piramid_core::error::ServerError;
use subtle::ConstantTimeEq;

use super::ApiError;

/// Middleware that answers 401 unless the request carries the key as a bearer token.
pub async fn require_api_key(
    State(key): State<Arc<ApiKey>>,
    request: Request,
    next: Next,
) -> Response {
    let presented = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(bearer_token);
    match presented {
        Some(token) if key_matches(&key, token) => next.run(request).await,
        _ => {
            tracing::debug!(
                target: "piramid::http",
                path = request.uri().path(),
                "request_unauthenticated"
            );
            unauthorized()
        }
    }
}

/// The token of a Bearer authorization header value. The scheme matches case-insensitively.
fn bearer_token(value: &str) -> Option<&str> {
    let (scheme, token) = value.split_once(' ')?;
    scheme
        .eq_ignore_ascii_case("bearer")
        .then_some(token.trim())
}

/// Compares the presented token with the key in constant time for tokens of equal length.
fn key_matches(key: &ApiKey, presented: &str) -> bool {
    key.expose().as_bytes().ct_eq(presented.as_bytes()).into()
}

/// A 401 response with a JSON body and a Bearer challenge.
fn unauthorized() -> Response {
    let mut response = ApiError::from(ServerError::AuthenticationFailed(
        "missing or invalid bearer key in the Authorization header".to_string(),
    ))
    .into_response();
    response
        .headers_mut()
        .insert(WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
    response
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "a failed assertion is the point of a test"
)]
mod tests {
    use super::*;

    #[test]
    fn the_bearer_scheme_is_case_insensitive_and_others_are_refused() {
        assert_eq!(bearer_token("Bearer abc"), Some("abc"));
        assert_eq!(bearer_token("bearer abc"), Some("abc"));
        assert_eq!(bearer_token("Basic abc"), None);
        assert_eq!(bearer_token("abc"), None);
    }

    #[test]
    fn only_the_exact_key_matches() {
        let key = ApiKey::new("secret-key".to_string()).unwrap();
        assert!(key_matches(&key, "secret-key"));
        assert!(!key_matches(&key, "secret-kez"));
        assert!(!key_matches(&key, "secret-key-longer"));
        assert!(!key_matches(&key, ""));
    }
}
