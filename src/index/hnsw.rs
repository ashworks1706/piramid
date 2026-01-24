use uuid::Uuid
use serde::{Serialize, Deserialize};

// HNSW (Hierarchical Navigable Small World) index configuration
// Derived from the original HNSW paper by Malkov and Yashunin
// https://arxiv.org/abs/1603.09320
// https://www.pinecone.io/learn/series/faiss/hnsw/
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
            ml: 16, // max number of connections per nodes
            ef_construction: 200, // size of the dynamic list for the construction phase
            ef_search: 50, // size of the dynamic list for the search phase
            distance_metric: "cosine".to_string(), // distance metric to use
            id: Uuid::new_v4(), // unique identifier for the index
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


fn random_layer(&self) -> usize{
    // exponential decay probability 
    // floor(-ln(uniform_random) * max_level)
    let r: f64 = rand::random();
    let level = (-r.ln() * (1.0 / self.config.ml as f64)).floor() as usize;
    level
}


fn insert(&mut self, id: Uuid, vector: Vec<f32>){
    // determine the layer for the new node 
    let layer = self.random_layer(); // this gives us a layer based on exponential decay

    // if first node, make it entry point and return 
    if self.start_node.is_none(){
        self.start_node = Some(id); // set entry point
        self.max_level = layer as isize; // we do this because levels are 0-indexed and this makes
                                         // sure max_level is always the highest level
        let node = HnswNode{ 
            id,
            vector,
            level: layer as isize,
        }; // create the node
        self.nodes.insert(id, node); // insert into the index
        return;
    }

    // otherwise, we need to find the best entry point for each layer down to 0
    // start from the highest layer of the current entry point


}

























