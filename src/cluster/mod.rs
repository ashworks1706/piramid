//! Cluster boundary.
//!
//! This module is intentionally a scaffold until Piramid has real distributed-system code to move
//! here. Future work should keep membership, node capability discovery, shard ownership,
//! replication policy, fan-out routing, and partial-result handling behind this boundary instead
//! of mixing them into runtime state, services, storage, or search.
