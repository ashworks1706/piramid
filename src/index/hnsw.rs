use uuid::Uuid;
use std::collections::{HashMap, HashSet, BinaryHeap};
use std::cmp::Ordering;
use crate::metrics::Metric;


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
    pub m: usize,  // max number of connections per node
    pub m_max: usize,  // max connections for layer 0 (typically 2*M)
    pub ef_construction: usize,  // size of the dynamic list for the construction phase
    pub ml: f32,  // layer multiplier: 1/ln(M)
    pub metric: Metric,  // similarity metric (converted to distance internally for HNSW)
}
// impl means we are implementing methods for the struct where each method has &self as first
// parameter, meaning it operates on an instance of the struct, similar to classes in other
// languages, all the content in that impl block are methods for HnswConfig, a user can use them
// by creating an instance of HnswConfig and calling the methods on it for example:
// let config = HnswConfig::default(); <- this creates a default config instance
// let m = config.m; <- this accesses the m field of the config instance
// but you might wonder where did 'Default' from impl Default for HnswConfig come from? 
// Default is a trait in Rust that provides a way to create default values for types,
// by implementing Default for HnswConfig, we are saying that HnswConfig can have a default value
// and we provide the implementation of how to create that default value in the fn default() method
impl Default for HnswConfig {
    fn default() -> Self {
        let m = 16;
        HnswConfig {
            m,
            m_max: m * 2,
            ef_construction: 200, // size of the dynamic list for the construction phase
            ml: 1.0 / (m as f32).ln(),  // layer multiplier: 1/ln(M)
            metric: Metric::Cosine,  // default to cosine similarity
        }
    }
}

#[derive(Debug, Clone)]
struct HnswNode{
    // connections[layer] = Vec of neighbor IDs at that layer
    // Layer 0 is at index 0
    connections: Vec<Vec<Uuid>>,
}

// Helper struct for priority queue during search
#[derive(Debug, Clone)]
struct SearchCandidate {
    id: Uuid,
    distance: f32,
}
// partial_eq means we can compare two SearchCandidate for equality based on distance
// PartialEq is a trait in Rust that allows you to define how two instances of a type
// are compared for equality, by implementing PartialEq for SearchCandidate, we are saying
// that SearchCandidate can be compared for equality and we provide the implementation of
// how to compare them in the fn eq() method
impl PartialEq for SearchCandidate {
    fn eq(&self, other: &Self) -> bool {
        self.distance == other.distance // equality based on distance
    }
}

impl Eq for SearchCandidate {}

// partial_ord means we can compare two SearchCandidate for ordering based on distance
// PartialOrd is a trait in Rust that allows you to define how two instances of a type
// are compared for ordering (less than, greater than, etc.), by implementing PartialOrd
// for SearchCandidate, we are saying that SearchCandidate can be compared for ordering
// and we provide the implementation of how to compare them in the fn partial_cmp() method
impl PartialOrd for SearchCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other)) // we order using the Ord implementation below
    }
}

// Ord is a trait in Rust that allows you to define a total ordering for a type,
// by implementing Ord for SearchCandidate, we are saying that SearchCandidate has a
// total ordering and we provide the implementation of how to compare them in the fn cmp() method
impl Ord for SearchCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        // For max heap, reverse ordering so closest (smallest distance) comes first
        other.distance.partial_cmp(&self.distance).unwrap_or(Ordering::Equal) 
        // Ordering::Equal means the two distances are equal
        // by doing this, we order SearchCandidate in descending order of distance. how? by 
        // comparing other to self instead of self to other, so the one with smaller distance
        // is considered "greater" in the context of a max-heap, thus it will be popped first
    }
}

// Main HNSW index structure
pub struct HnswIndex{
    config: HnswConfig, // configuration parameters, for example: m, ef_construction, ml, metric
    nodes: HashMap<Uuid, HnswNode>,
    max_level: isize,
    start_node: Option<Uuid>,
}

impl HnswIndex{
    pub fn new(config: HnswConfig) -> Self{
        HnswIndex{
            config,
            nodes: HashMap::new(),
            max_level: -1,
            start_node: None,
        }
    }
    // Generate a random layer for a new node based on exponential decay why? because 
    // in HNSW, higher layers have exponentially fewer nodes, so we want to assign layers
    // to new nodes in a way that reflects this distribution
    fn random_layer(&self) -> usize{
        // exponential decay probability 
        // floor(-ln(uniform_random) * ml)
        let r: f32 = rand::random();
        (-r.ln() * self.config.ml).floor() as usize // this basically gives us a layer based on
                                                    // exponential decay
    }

    // Insert a node with access to vector storage for distance calculations
    pub fn insert(&mut self, id: Uuid, vector: &[f32], vectors: &HashMap<Uuid, Vec<f32>>){
        // in hnsw, we add nodes one at a time, connecting them to existing nodes
        // first, we need to create the node and determine its level
        // determine the layer for the new node 
        let layer = self.random_layer(); // this gives us a layer based on exponential decay

        // if first node, make it entry point and return 
        if self.start_node.is_none(){
            self.start_node = Some(id); // set entry point
            self.max_level = layer as isize; // this makes sure max_level is always the highest level
            let node = HnswNode{ 
                connections: vec![Vec::new(); layer + 1], // this creates empty connections for
                                                          // each layer
            }; // create the node
            self.nodes.insert(id, node); // insert into the index
            return;
        }

        // otherwise, we need to find the best entry point for each layer down to 0
        // we do this by greedy search
        // start from the highest layer of the current entry point
        let mut current_entry = vec![self.start_node.unwrap()];
        
        // Search from top layer down to target layer (layer + 1)
        for lc in ((layer as isize + 1)..=self.max_level).rev() {
            current_entry = self.search_layer(vector, &current_entry, 1, lc as usize, vectors);
        }

        // Insert and connect at each layer from target down to 0
        for lc in (0..=layer).rev() { // 0..=layer means we go from layer down to 0 since we are
                                      // doing rev()
            // Search for ef_construction nearest neighbors at this layer
            current_entry = self.search_layer( // ef_construction means we want to find this many
                                               // neighbors
                vector,
                &current_entry,
                self.config.ef_construction,
                lc,
                vectors
            );

            // Select M best neighbors (or M_max for layer 0)
            let m = if lc == 0 { self.config.m_max } else { self.config.m };
            let neighbors = self.select_neighbors(&current_entry, m, vectors, vector);

            // Add bidirectional connections
            // we do this by adding edges in both directions between the new node and its neighbors
            // at the current layer since HNSW uses undirected edges, undirectec edges mean that if node A
            // is connected to node B, then node B is also connected to node A
            for &neighbor_id in &neighbors {
                // Add edge from new node to neighbor
                if let Some(node) = self.nodes.get_mut(&id) {
                    if lc < node.connections.len() {
                        node.connections[lc].push(neighbor_id);
                    }
                }

                // Add edge from neighbor to new node
                if let Some(neighbor) = self.nodes.get_mut(&neighbor_id) { // get mutable reference
                                                                           // to neighbor why?
                                                                           // because we want to
                                                                           // modify its
                                                                           // connections
                    if lc < neighbor.connections.len() {
                        neighbor.connections[lc].push(id);

                        // Prune connections if neighbor exceeds max why? because HNSW limits the
                        // number of connections per node to maintain efficiency
                        if neighbor.connections[lc].len() > m {
                            // Clone the connections and neighbor vector to avoid borrow issues
                            let neighbor_connections = neighbor.connections[lc].clone();
                            let neighbor_vec = vectors.get(&neighbor_id).unwrap().clone();
                            
                            let pruned = self.select_neighbors(
                                &neighbor_connections,
                                m,
                                vectors,
                                &neighbor_vec
                            );
                            
                            if let Some(neighbor) = self.nodes.get_mut(&neighbor_id) {
                                if lc < neighbor.connections.len() {
                                    neighbor.connections[lc] = pruned;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Create and insert the new node
        let new_node = HnswNode{
            connections: vec![Vec::new(); layer + 1],
        };
        self.nodes.insert(id, new_node);

        // Update entry point if this node is at a higher layer
        if layer as isize > self.max_level {
            self.max_level = layer as isize;
            self.start_node = Some(id);
        }
    }

    // Public search function - returns k nearest neighbor IDs
    // how do we even search in hnsw, what is the core condition for determining nearest neighbour?
    // the core condition is based on distance metric, we want to find nodes that are closest
    // to the query vector based on the configured distance metric (e.g., cosine, euclidean, dot
    // product)
    // searching is done in two phases:
    // 1. Greedy search from top layer down to layer 1 to find entry point for layer 0
    // 2. Search layer 0 with ef parameter to find k nearest neighbors
    // ef is a parameter that controls the accuracy/speed tradeoff during search
    pub fn search(&self, query: &[f32], k: usize, ef: usize, vectors: &HashMap<Uuid, Vec<f32>>) -> Vec<Uuid> {
        if self.start_node.is_none() {
            return Vec::new();
        }

        let ep = self.start_node.unwrap();
        let mut current_nearest = vec![ep];

        // Search from top layer down to layer 1
        for lc in (1..=self.max_level as usize).rev() {
            current_nearest = self.search_layer(query, &current_nearest, 1, lc, vectors);
        }

        // Search layer 0 with ef
        current_nearest = self.search_layer(query, &current_nearest, ef.max(k), 0, vectors);
        
        // Return top k
        current_nearest.truncate(k);
        current_nearest
    }

    // Search within a specific layer - returns nearest neighbor IDs sorted by distance
    fn search_layer(
        &self,
        query: &[f32],
        entry_points: &[Uuid],
        num_closest: usize,
        level: usize,
        vectors: &HashMap<Uuid, Vec<f32>>,
    ) -> Vec<Uuid> {
        let mut visited = HashSet::new();
        let mut candidates = BinaryHeap::new();
        let mut nearest = BinaryHeap::new();

        // Initialize with entry points
        for &ep in entry_points {
            if let Some(ep_vector) = vectors.get(&ep) {
                let dist = self.distance(query, ep_vector);
                candidates.push(SearchCandidate { id: ep, distance: dist });
                nearest.push(SearchCandidate { id: ep, distance: dist });
                visited.insert(ep);
            }
        }

        // we track furthest distance by looking at the top of the nearest heap (since it's a
        // max-heap)
        let mut furthest_distance = nearest.peek().map(|c| c.distance).unwrap_or(f32::INFINITY);

        // Greedy search within the layer basically, greedy search means we always explore the
        // closest candidate first
        while let Some(candidate) = candidates.pop() {
            if candidate.distance > furthest_distance {
                break;
            }

            // Explore neighbors at this level
            if let Some(node) = self.nodes.get(&candidate.id) {
                if level < node.connections.len() {
                    for &neighbor_id in &node.connections[level] {
                        if visited.insert(neighbor_id) { // only proceed if not visited
                            if let Some(neighbor_vector) = vectors.get(&neighbor_id) { 
                                let dist = self.distance(query, neighbor_vector);
                                
                                // If this neighbor is closer than the furthest in nearest, add it
                                if dist < furthest_distance || nearest.len() < num_closest {
                                    candidates.push(SearchCandidate { id: neighbor_id, distance: dist });
                                    nearest.push(SearchCandidate { id: neighbor_id, distance: dist });
                                    
                                    if nearest.len() > num_closest {
                                        nearest.pop(); // remove furthest
                                    }
                                    
                                    // Update furthest distance
                                    furthest_distance = nearest.peek().map(|c| c.distance).unwrap_or(f32::INFINITY);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Convert heap to sorted vector (closest first)
        let mut result: Vec<_> = nearest.into_iter().collect();
        result.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(Ordering::Equal)); // sort
                                                                                               // ascending
                                                                                               // by
                                                                                               // distance
        result.into_iter().map(|c| c.id).collect() // return only IDs
    }

    // Select M best neighbors using simple heuristic
    fn select_neighbors(
        &self,
        candidates: &[Uuid],
        m: usize,
        vectors: &HashMap<Uuid, Vec<f32>>,
        query: &[f32],
    ) -> Vec<Uuid> {
        if candidates.len() <= m {
            return candidates.to_vec();
        }

        // Simple heuristic: select M closest by distance
        let mut distances: Vec<_> = candidates
            .iter()
            .filter_map(|&id| {
                vectors.get(&id).map(|vec| {
                    let dist = self.distance(query, vec);
                    (id, dist)
                })
            })
            .collect();

        distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
        distances.truncate(m);
        distances.into_iter().map(|(id, _)| id).collect()
    }

    fn get_neighbors_at_level(&self, node_id: &Uuid, level: usize) -> Vec<Uuid> {
        // get neighbors at the current level
        if let Some(node) = self.nodes.get(node_id) {
            if level < node.connections.len() {
                return node.connections[level].clone();
            }
        }
        Vec::new()
    }

    // distance function that calculates using configured metric
    fn distance(&self, a: &[f32], b: &[f32]) -> f32 {
        // HNSW works with distances (lower = better)
        // But our metrics return similarity scores (higher = better for Cosine/Dot)
        // So we need to invert for those metrics
        match self.config.metric {
            Metric::Cosine => 1.0 - self.config.metric.calculate(a, b),
            Metric::DotProduct => 1.0 - self.config.metric.calculate(a, b),
            Metric::Euclidean => self.config.metric.calculate(a, b),
        }
    }

    // Remove a node from the index
    pub fn remove(&mut self, id: &Uuid) {
        if let Some(node) = self.nodes.remove(id) {
            // Remove all connections to this node from other nodes
            // we do this by iterating through all layers and removing any references
            // to the node being removed and updating the connections of neighboring nodes
            // accordingly
            for layer in 0..node.connections.len() {
                for &neighbor_id in &node.connections[layer] {
                    if let Some(neighbor) = self.nodes.get_mut(&neighbor_id) {
                        if layer < neighbor.connections.len() {
                            neighbor.connections[layer].retain(|&nid| nid != *id);
                        }
                    }
                }
            }

            // Update entry point if needed
            if self.start_node == Some(*id) {
                self.start_node = self.nodes.keys().next().copied();
                self.max_level = self.nodes.values()
                    .map(|n| n.connections.len() as isize - 1)
                    .max()
                    .unwrap_or(-1);
            }
        }
    }

    // Get statistics about the index
    // we do this by iterating through all nodes and collecting data such as
    // total number of nodes, max layer, size of each layer, average connections per node
    pub fn stats(&self) -> HnswStats {
        let total_nodes = self.nodes.len();
        let mut layer_sizes = vec![0; (self.max_level + 1) as usize];
        let mut total_connections = 0;

        for node in self.nodes.values() {
            for (layer, connections) in node.connections.iter().enumerate() {
                if layer < layer_sizes.len() {
                    layer_sizes[layer] += 1;
                }
                total_connections += connections.len();
            }
        }

        HnswStats {
            total_nodes,
            max_layer: self.max_level,
            layer_sizes,
            avg_connections: if total_nodes > 0 {
                total_connections as f32 / total_nodes as f32
            } else {
                0.0
            },
        }
    }
}

// Statistics about the HNSW index
#[derive(Debug)]
pub struct HnswStats {
    pub total_nodes: usize,
    pub max_layer: isize,
    pub layer_sizes: Vec<usize>,
    pub avg_connections: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hnsw_insert_and_search() {
        let config = HnswConfig::default();
        let mut index = HnswIndex::new(config);
        let mut vectors = HashMap::new();

        // Insert some test vectors
        let ids: Vec<Uuid> = (0..10).map(|_| Uuid::new_v4()).collect();
        for (i, &id) in ids.iter().enumerate() {
            let vector = vec![i as f32, (i * 2) as f32, (i * 3) as f32];
            vectors.insert(id, vector.clone());
            index.insert(id, &vector, &vectors);
        }

        // Search for something close to vector 5
        let query = vec![5.0, 10.0, 15.0];
        let results = index.search(&query, 3, 50, &vectors);

        assert!(results.len() <= 3);
        assert!(results.contains(&ids[5])); // Should find the closest match
    }

    #[test]
    fn test_hnsw_empty_search() {
        let config = HnswConfig::default();
        let index = HnswIndex::new(config);
        let vectors = HashMap::new();

        let query = vec![1.0, 2.0, 3.0];
        let results = index.search(&query, 5, 50, &vectors);

        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_hnsw_remove() {
        let config = HnswConfig::default();
        let mut index = HnswIndex::new(config);
        let mut vectors = HashMap::new();

        let id = Uuid::new_v4();
        let vector = vec![1.0, 2.0, 3.0];
        vectors.insert(id, vector.clone());
        index.insert(id, &vector, &vectors);

        assert_eq!(index.nodes.len(), 1);

        index.remove(&id);
        assert_eq!(index.nodes.len(), 0);
    }
}






















