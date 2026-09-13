//! Model execution.
//!
//! Depends on hardware::gpu for the device runtime and on fusion for the seam retrieval enters
//! through, and on nothing in the retrieval stack.

pub mod architecture;
pub mod backends;
pub mod batching;
pub mod forward;
pub mod kv_cache;
pub mod manager;
pub mod sampling;
pub mod tokenizer;

pub use manager::InferenceManager;
