//! Per-collection size ceilings.

use serde::{Deserialize, Serialize};

/// Ceilings a collection refuses writes beyond.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct LimitsConfig {
    /// Max number of vectors in a collection. None is unlimited.
    pub max_vectors: Option<usize>,
    /// Max on-disk bytes for a collection. None is unlimited.
    pub max_bytes: Option<u64>,
    /// Max serialized bytes per vector. None is unlimited.
    pub max_vector_bytes: Option<usize>,
}
