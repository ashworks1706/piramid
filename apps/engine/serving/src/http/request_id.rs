//! Request id assignment and per-request logging.

use axum::{body::Body, http::Request, middleware::Next, response::Response};
use std::time::Instant;
use uuid::Uuid;

/// Request ID stored in request extensions.
#[derive(Clone, Debug)]
pub struct RequestId(pub String);

/// Assigns a request ID, adds it to extensions, and echoes it in the response header.
pub async fn assign_request_id(mut req: Request<Body>, next: Next) -> Response {
    let req_id = req
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty())
        .map_or_else(|| Uuid::new_v4().to_string(), ToOwned::to_owned);
    let method = req.method().to_string();
    let path = req
        .uri()
        .path_and_query()
        .map_or_else(|| req.uri().path().to_string(), |v| v.as_str().to_string());
    let request_content_length = req
        .headers()
        .get(axum::http::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok());
    let user_agent = req
        .headers()
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-")
        .to_string();
    let forwarded_for = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-")
        .to_string();
    let start = Instant::now();
    req.extensions_mut().insert(RequestId(req_id.clone()));

    let mut res = next.run(req).await;
    let status = res.status().as_u16();
    let elapsed_ms = start.elapsed().as_millis();
    let response_content_length = res
        .headers()
        .get(axum::http::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok());
    if let Ok(header) = req_id.parse() {
        res.headers_mut().insert("x-request-id", header);
    }

    if status >= 500 {
        tracing::error!(
            target: "piramid::http",
            request_id = req_id.as_str(),
            method = method.as_str(),
            path = path.as_str(),
            status = status,
            elapsed_ms = elapsed_ms as u64,
            request_content_length = request_content_length,
            response_content_length = response_content_length,
            user_agent = user_agent.as_str(),
            forwarded_for = forwarded_for.as_str(),
            "http_request_failed"
        );
    } else if status >= 400 {
        tracing::warn!(
            target: "piramid::http",
            request_id = req_id.as_str(),
            method = method.as_str(),
            path = path.as_str(),
            status = status,
            elapsed_ms = elapsed_ms as u64,
            request_content_length = request_content_length,
            response_content_length = response_content_length,
            user_agent = user_agent.as_str(),
            forwarded_for = forwarded_for.as_str(),
            "http_request_client_error"
        );
    } else {
        tracing::info!(
            target: "piramid::http",
            request_id = req_id.as_str(),
            method = method.as_str(),
            path = path.as_str(),
            status = status,
            elapsed_ms = elapsed_ms as u64,
            request_content_length = request_content_length,
            response_content_length = response_content_length,
            user_agent = user_agent.as_str(),
            forwarded_for = forwarded_for.as_str(),
            "http_request"
        );
    }
    res
}
