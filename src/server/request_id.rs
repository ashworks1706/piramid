use axum::{http::Request, response::Response, middleware::Next, body::Body};
use uuid::Uuid;

/// Request ID stored in request extensions.
#[derive(Clone, Debug)]
pub struct RequestId(pub String);

/// Middleware that assigns a request ID, adds it to extensions, and echoes it in the response header.
pub async fn assign_request_id(mut req: Request<Body>, next: Next) -> Response {
    let req_id = Uuid::new_v4().to_string();
    req.extensions_mut().insert(RequestId(req_id.clone()));

    let mut res = next.run(req).await;
    res.headers_mut().insert("x-request-id", req_id.parse().unwrap());
    res
}
