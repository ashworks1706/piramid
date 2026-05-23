pub mod handlers; // endpoint logic
pub mod helpers; // utility functions and macros
pub mod request_id;
pub mod routes; // wires handlers to URL paths
pub mod types; // request/response JSON types

pub use helpers::{json_to_metadata, metadata_to_json};
pub use routes::create_router;
