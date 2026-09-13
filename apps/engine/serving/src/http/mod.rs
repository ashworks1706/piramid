//! HTTP transport: routes, handlers, auth, rate limiting, and the serve loop.

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
