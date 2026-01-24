use uuid::Uuid'
use serde::{Serialize, Deserialize};

pub struct HnswConfig{
    pub ml: usize,
    pub ef_construction: usize,
    pub ef_search: usize,
    pub distance_metric: String,
    pub id: Uuid,
}

impl Default for HnswConfig {
    fn default() -> Self {
        HnswConfig {
            ml: 16,
            ef_construction: 200,
            ef_search: 50,
            distance_metric: "cosine".to_string(),
            id: Uuid::new_v4(),
        }
    }
}
