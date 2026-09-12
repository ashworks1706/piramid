//! HTTP transport, use-case orchestration, and shared runtime state.
//!
//! - [http] holds axum routes, handlers and request ids. Handlers parse the request, call a
//!   service, and serialize the result.
//! - [services] holds admission checks, lock acquisition, metrics, and the canonical API DTOs.
//!   Durability belongs to the collection layer, not here.
//! - [state] holds [AppState], the process-wide shared state, and [disk] the pressure watcher.
//! - [machine] holds the host readings, sampled on a background thread.
//! - [cluster] holds node identity and routing. Local-only today.

pub mod cluster;
pub mod disk;
pub mod http;
pub mod machine;
pub mod services;
pub mod state;

pub use http::create_router;
pub use state::{AppState, SharedState};
