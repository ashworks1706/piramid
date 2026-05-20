// Search mode configuration

use serde::{Deserialize, Serialize};

// mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SearchMode {
    // K-nearest neighbors return top k results
    #[default]
    KNN,
    // Range search return all within distance threshold
    Range,
}

// Range search parameters
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RangeSearchParams {
    // Maximum distance threshold
    pub max_distance: f32,

    // Maximum number of results (None = unlimited)
    pub max_results: Option<usize>,
}

impl RangeSearchParams {
    pub fn new(max_distance: f32) -> Self {
        RangeSearchParams {
            max_distance,
            max_results: None,
        }
    }

    pub fn with_limit(max_distance: f32, max_results: usize) -> Self {
        RangeSearchParams {
            max_distance,
            max_results: Some(max_results),
        }
    }
}
