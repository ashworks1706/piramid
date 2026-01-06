//! Server module - HTTP API for Piramid
//!
//! This is the "web layer" - it turns our Rust library into a network service.
//! The pattern: Library (pure logic) → Server (HTTP wrapper) → Clients (SDK, dashboard)
//!
//! ## Module structure
//! - `state.rs` - shared app state (thread-safe collection storage)
//! - `types.rs` - request/response JSON types
//! - `handlers.rs` - the actual endpoint logic
//! - `routes.rs` - wires handlers to URL paths

pub mod state;
pub mod types;
pub mod handlers;
pub mod routes;

// Re-export for convenience
pub use state::{AppState, SharedState};
pub use routes::create_router;
