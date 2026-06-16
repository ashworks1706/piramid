use axum::{body::Body, http::Request, middleware::Next, response::Response};
use std::time::Instant;
use uuid::Uuid;

/// Request ID stored in request extensions.
#[derive(Clone, Debug)]
pub struct RequestId(pub String); // Simple wrapper around a string to represent a request ID

/// Middleware that assigns a request ID, adds it to extensions, and echoes it in the response header.
pub async fn assign_request_id(mut req: Request<Body>, next: Next) -> Response {
    let req_id = Uuid::new_v4().to_string();
    let method = req.method().to_string();
    let path = req
        .uri()
        .path_and_query()
        .map(|v| v.as_str().to_string())
        .unwrap_or_else(|| req.uri().path().to_string());
    let user_agent = req
        .headers()
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-")
        .to_string();
    let start = Instant::now();
    req.extensions_mut().insert(RequestId(req_id.clone()));

    let mut res = next.run(req).await;
    let status = res.status().as_u16();
    let elapsed_ms = start.elapsed().as_millis();
    res.headers_mut()
        .insert("x-request-id", req_id.parse().unwrap());
    tracing::info!(
        target: "piramid::http",
        request_id = req_id.as_str(),
        method = method.as_str(),
        path = path.as_str(),
        status = status,
        elapsed_ms = elapsed_ms as u64,
        user_agent = user_agent.as_str(),
        "http_request"
    );
    res
}
