use axum::{body::Body, http::Request, middleware::Next, response::Response};
use uuid::Uuid;

/// Request ID stored in request extensions.
#[derive(Clone, Debug)]
pub struct RequestId(pub String); // Simple wrapper around a string to represent a request ID

/// Middleware that assigns a request ID, adds it to extensions, and echoes it in the response header.
pub async fn assign_request_id(mut req: Request<Body>, next: Next) -> Response {
    let req_id = Uuid::new_v4().to_string();
    req.extensions_mut().insert(RequestId(req_id.clone()));

    let mut res = next.run(req).await;
    res.headers_mut()
        .insert("x-request-id", req_id.parse().unwrap());
    res
}
