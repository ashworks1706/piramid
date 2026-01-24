use uuid::Uuid
use serde::{Serialize, Deserialize};

// HNSW (Hierarchical Navigable Small World) index configuration
// Derived from the original HNSW paper by Malkov and Yashunin
// https://arxiv.org/abs/1603.09320
// HNSW is an efficient algorithm for approximate nearest neighbor search in high-dimensional
// spaces, it wokrs by building a multi-layer graph structure where each layer is a navigable small
// world graph. The top layers contain fewer nodes and provide long-range connections, while the lower layers contain more nodes and provide local connections.
// During search, the algorithm starts at the top layer and traverses down to the lower layers,
// using the connections to quickly find approximate nearest neighbors
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

#(derive(Debug, Clone))
struct HnswNode{
    id: Uuid,
    vector: Vec<f32>,
    level: isize,
}

pub struct HnswIndex{
    config: HnswConfig,
    nodes: Hashmap<Uuid, HnswNode>,
    max_level: isize,
    start_node: Option<Uuid>,
}

impl HnswIndex{
    pub fn new(config: HnswConfig) -> Self{
        HnswIndex{
            config : HnswConfig,
            nodes: Hashmap::new(),
            max_level: -1,
            start_node: None,
        }
    }
}












