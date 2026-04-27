
pub mod state; // shared app state (thread-safe collection storage)
pub mod types; // request/response JSON types
pub mod handlers; // endpoint logic
pub mod routes; // wires handlers to URL paths
pub mod helpers; // utility functions and macros
pub mod metrics; // application metrics collection
pub mod request_id; 

pub use state::{AppState, SharedState};
pub use routes::create_router;
pub use helpers::{json_to_metadata, metadata_to_json};
