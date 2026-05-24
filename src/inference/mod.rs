//! Inference boundary.
//!
//! This module is intentionally a scaffold until Piramid has real local inference code to move
//! here. Future work should keep model placement, local inference adapters, request batching,
//! token streaming, KV-cache ownership, and OpenAI-compatible inference surfaces behind this
//! boundary instead of mixing them into HTTP handlers, services, storage, or search.
