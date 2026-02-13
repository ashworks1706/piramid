// Library (pure logic) → Server (HTTP wrapper) → Clients (SDK, dashboard)
// ## Module structure
// - `state.rs` - shared app state (thread-safe collection storage)
// - `types.rs` - request/response JSON types
// - `handlers.rs` - the actual endpoint logic
// - `routes.rs` - wires handlers to URL paths
// - `helpers.rs` - utility functions and macros

pub mod state;
pub mod types;
pub mod handlers;
pub mod routes;
pub mod helpers;
pub mod metrics;
pub mod request_id;

pub use state::{AppState, SharedState};
pub use routes::create_router;
pub use helpers::{json_to_metadata, metadata_to_json};
