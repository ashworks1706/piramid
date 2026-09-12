//! HTTP transport: routes, handlers, request ids, error mapping, the scrape endpoint,
//! authentication, rate limiting, and the serve loop with graceful shutdown.
//!
//! Handlers parse the request, call a service, and serialize the result. DTOs live in
//! services::api.

pub mod auth;
pub mod error;
pub mod handlers;
pub mod prometheus;
pub mod rate_limit;
pub mod request_id;
pub mod routes;
pub mod serve;

pub use error::{ApiError, ApiResult};
pub use routes::create_router;
