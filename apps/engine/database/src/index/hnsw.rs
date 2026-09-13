//! Hierarchical navigable small world index: a layered proximity graph searched greedily.

use piramid_hardware::compute::{strategies::for_mode, DistanceKernels, Metric};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};
use uuid::Uuid;

use crate::index::{MetadataReader, VectorReader};
use piramid_core::config::HnswConfig;
use piramid_core::error::{IndexError, Result};

/// Graph shape and occupancy, for the admin surface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnswStats {
    /// Live nodes, excluding tombstones.
    pub total_nodes: usize,
    /// Highest layer index in use, -1 when empty.
    pub max_layer: isize,
    /// Node count per layer, lowest first.
    pub layer_sizes: Vec<usize>,
    /// Nodes removed but still linked for traversal.
    pub tombstones: usize,
    /// Mean edges per live node at layer 0. None when there are no live nodes.
    pub avg_connections: Option<f32>,
    /// Approximate resident size.
    pub memory_usage_bytes: usize,
}

/// Total order over distances, with NaN after every number.
fn cmp_scores(a: f32, b: f32) -> Ordering {
    match (a.is_nan(), b.is_nan()) {
        (false, false) => a.partial_cmp(&b).unwrap_or(Ordering::Equal),
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HnswNode {
    /// Neighbours per layer, layer 0 first.
    connections: Vec<Vec<Uuid>>,
    /// Deleted, with edges kept so traversal stays connected.
    tombstone: bool,
}

#[derive(Debug, Clone)]
struct SearchCandidate {
    id: Uuid,
    distance: f32,
}

struct SearchContext<'a> {
    vectors: &'a dyn VectorReader,
    filter: Option<&'a piramid_core::metadata::Filter>,
    metadatas: &'a dyn MetadataReader,
    /// Resolved once per operation and reused for every distance in the traversal.
    kernels: &'a dyn DistanceKernels,
}
impl PartialEq for SearchCandidate {
    fn eq(&self, other: &Self) -> bool {
        self.distance == other.distance
    }
}

impl Eq for SearchCandidate {}

impl PartialOrd for SearchCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SearchCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reversed, so the max-heap orders the closest first.
        cmp_scores(other.distance, self.distance)
    }
}

/// A layered proximity graph over vector ids, with deleted nodes kept as tombstones.
#[derive(Clone, Serialize, Deserialize)]
pub struct HnswIndex {
    config: HnswConfig,
    nodes: HashMap<Uuid, HnswNode>,
    max_level: isize,
    start_node: Option<Uuid>,
}

impl HnswIndex {
    /// An empty graph with the given parameters.
    pub fn new(config: HnswConfig) -> Self {
        HnswIndex {
            config,
            nodes: HashMap::new(),
            max_level: -1,
            start_node: None,
        }
    }

    fn is_tombstone(&self, id: &Uuid) -> bool {
        self.nodes.get(id).is_some_and(|n| n.tombstone)
    }

    fn mark_tombstone(&mut self, id: &Uuid) {
        if let Some(node) = self.nodes.get_mut(id) {
            node.tombstone = true;
        }
    }
    /// Draw a layer for a new node from an exponentially decaying distribution.
    fn random_layer(&self) -> usize {
        // floor(-ln(uniform) * ml)
        let r: f32 = rand::random();
        (-r.ln() * self.config.ml).floor() as usize
    }

    /// Insert an id, linking it into each layer it occupies.
    pub fn insert(&mut self, id: Uuid, vector: &[f32], vectors: &dyn VectorReader) -> Result<()> {
        let kernels = for_mode(self.config.mode)?;
        let empty_meta: HashMap<Uuid, piramid_core::metadata::Metadata> = HashMap::new();
        let search_context = SearchContext {
            vectors,
            filter: None,
            metadatas: &empty_meta,
            kernels,
        };
        let layer = self.random_layer();

        // The first node becomes the entry point and has nothing to link to.
        let Some(entry_point) = self.start_node else {
            self.start_node = Some(id);
            self.max_level = layer as isize;
            self.nodes.insert(
                id,
                HnswNode {
                    connections: vec![Vec::new(); layer + 1],
                    tombstone: false,
                },
            );
            return Ok(());
        };

        // Greedy descent from the entry point to the top layer of the new node.
        let mut current_entry = vec![entry_point];

        for lc in ((layer as isize + 1)..=self.max_level).rev() {
            current_entry = self.search_layer(
                vector,
                &current_entry,
                1,
                lc as usize,
                &search_context,
                true,
            )?;
        }

        // Connect from the target layer down to 0. Connections are staged and applied after
        // pruning.
        let mut pending_connections = vec![Vec::new(); layer + 1];
        for lc in (0..=layer).rev() {
            current_entry = self.search_layer(
                vector,
                &current_entry,
                self.config.ef_construction,
                lc,
                &search_context,
                true,
            )?;

            // Layer 0 allows M_max edges; higher layers allow M.
            let m = if lc == 0 {
                self.config.m_max
            } else {
                self.config.m
            };
            let neighbors = self.select_neighbors(&current_entry, m, vectors, vector, kernels)?;

            // Edges are undirected, so each link is written in both directions.
            for &neighbor_id in &neighbors {
                if lc < pending_connections.len() {
                    pending_connections[lc].push(neighbor_id);
                }

                if let Some(neighbor) = self.nodes.get_mut(&neighbor_id) {
                    if lc < neighbor.connections.len() {
                        neighbor.connections[lc].push(id);

                        // A neighbour over the degree cap is pruned.
                        if neighbor.connections[lc].len() > m {
                            let neighbor_connections = neighbor.connections[lc].clone();
                            let neighbor_vec = vectors
                                .get(&neighbor_id)
                                .ok_or_else(|| {
                                    IndexError::SearchFailed(format!(
                                        "HNSW neighbour {neighbor_id} is missing from vector storage"
                                    ))
                                })?
                                .to_vec();

                            let pruned = self.select_neighbors(
                                &neighbor_connections,
                                m,
                                vectors,
                                &neighbor_vec,
                                kernels,
                            )?;

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

        let new_node = HnswNode {
            connections: pending_connections,
            tombstone: false,
        };
        self.nodes.insert(id, new_node);

        // A node above the current entry point becomes the new entry point.
        if layer as isize > self.max_level {
            self.max_level = layer as isize;
            self.start_node = Some(id);
        }
        Ok(())
    }

    /// Find the k nearest neighbours of the query, widening to ef candidates at layer 0.
    pub fn search(
        &self,
        query: &[f32],
        k: usize,
        ef: usize,
        vectors: &dyn VectorReader,
        filter: Option<&piramid_core::metadata::Filter>,
        metadatas: &dyn MetadataReader,
    ) -> Result<Vec<Uuid>> {
        let Some(ep) = self.start_node else {
            return Ok(Vec::new());
        };
        let kernels = for_mode(self.config.mode)?;

        if vectors.get(&ep).is_none() {
            return Err(IndexError::SearchFailed(format!(
                "HNSW entry point {ep} is missing from vector storage"
            ))
            .into());
        }
        let mut current_nearest = vec![ep];

        let search_context = SearchContext {
            vectors,
            filter,
            metadatas,
            kernels,
        };

        for lc in (1..=self.max_level as usize).rev() {
            current_nearest =
                self.search_layer(query, &current_nearest, 1, lc, &search_context, true)?;
        }

        current_nearest = self.search_layer(
            query,
            &current_nearest,
            ef.max(k),
            0,
            &search_context,
            false,
        )?;

        let mut filtered: Vec<Uuid> = current_nearest
            .into_iter()
            .filter(|id| !self.is_tombstone(id))
            .collect();
        filtered.truncate(k);
        Ok(filtered)
    }

    /// Walk one layer, returning neighbour ids nearest-first.
    ///
    /// With admit_all set, every node is admitted regardless of filter or tombstone. The descent
    /// through the upper layers sets it, and the layer-0 call clears it.
    fn search_layer(
        &self,
        query: &[f32],
        entry_points: &[Uuid],
        num_closest: usize,
        level: usize,
        context: &SearchContext<'_>,
        admit_all: bool,
    ) -> Result<Vec<Uuid>> {
        let mut visited = HashSet::new();
        let mut candidates = BinaryHeap::new();
        let mut nearest = BinaryHeap::new();

        for &ep in entry_points {
            let Some(ep_vector) = context.vectors.get(&ep) else {
                continue;
            };
            let dist = self.distance(query, ep_vector, context.kernels)?;
            // Traversal continues through a node whether or not it is admitted.
            candidates.push(SearchCandidate {
                id: ep,
                distance: dist,
            });
            if admit_all || (!self.is_tombstone(&ep) && self.passes_filter(&ep, context)) {
                nearest.push(SearchCandidate {
                    id: ep,
                    distance: dist,
                });
            }
            visited.insert(ep);
        }

        let mut furthest_distance = nearest.peek().map_or(f32::INFINITY, |c| c.distance);

        // Explore the closest candidate first, stopping once nothing closer remains.
        while let Some(candidate) = candidates.pop() {
            if candidate.distance > furthest_distance {
                break;
            }

            let Some(node) = self.nodes.get(&candidate.id) else {
                continue;
            };
            if level >= node.connections.len() {
                continue;
            }
            for &neighbor_id in &node.connections[level] {
                if !visited.insert(neighbor_id) {
                    continue;
                }
                let Some(neighbor_vector) = context.vectors.get(&neighbor_id) else {
                    continue;
                };
                let dist = self.distance(query, neighbor_vector, context.kernels)?;
                let admissible = admit_all
                    || (!self.is_tombstone(&neighbor_id)
                        && self.passes_filter(&neighbor_id, context));

                if dist < furthest_distance || nearest.len() < num_closest {
                    candidates.push(SearchCandidate {
                        id: neighbor_id,
                        distance: dist,
                    });
                    if admissible {
                        nearest.push(SearchCandidate {
                            id: neighbor_id,
                            distance: dist,
                        });

                        if nearest.len() > num_closest {
                            nearest.pop();
                        }

                        furthest_distance = nearest.peek().map_or(f32::INFINITY, |c| c.distance);
                    }
                }
            }
        }

        let mut result: Vec<_> = nearest.into_iter().collect();
        result.sort_by(|a, b| b.cmp(a)); // Nearest first.
        Ok(result.into_iter().map(|c| c.id).collect())
    }

    // A metadata cache miss is admitted, and search::engine filters the resolved document.
    fn passes_filter(&self, id: &Uuid, context: &SearchContext<'_>) -> bool {
        context
            .filter
            .is_none_or(|filter| filter.may_match(context.metadatas.get(id)))
    }

    /// Pick the m closest candidates by distance only, with no diversity heuristic.
    fn select_neighbors(
        &self,
        candidates: &[Uuid],
        m: usize,
        vectors: &dyn VectorReader,
        query: &[f32],
        kernels: &dyn DistanceKernels,
    ) -> Result<Vec<Uuid>> {
        if candidates.len() <= m {
            return Ok(candidates.to_vec());
        }

        let mut distances = Vec::with_capacity(candidates.len());
        for &id in candidates {
            if self.is_tombstone(&id) {
                continue;
            }
            if let Some(vec) = vectors.get(&id) {
                distances.push((id, self.distance(query, vec, kernels)?));
            }
        }

        distances.sort_by(|a, b| cmp_scores(a.1, b.1));
        distances.truncate(m);
        Ok(distances.into_iter().map(|(id, _)| id).collect())
    }

    /// Distance under the configured metric, normalized so smaller is always nearer.
    fn distance(&self, a: &[f32], b: &[f32], kernels: &dyn DistanceKernels) -> Result<f32> {
        let score = self.config.metric.calculate(a, b, kernels)?;
        Ok(match self.config.metric {
            // Similarity metrics score higher for nearer, so they are inverted.
            Metric::Cosine | Metric::DotProduct => 1.0 - score,
            Metric::Euclidean => score,
        })
    }

    /// Tombstone a node, keeping its edges so traversal stays connected.
    pub fn remove(&mut self, id: &Uuid) {
        if !self.nodes.contains_key(id) {
            return;
        }
        self.mark_tombstone(id);

        if self.start_node == Some(*id) {
            self.start_node = self
                .nodes
                .iter()
                .find(|(_, n)| !n.tombstone)
                .map(|(k, _)| *k);
            self.max_level = self
                .nodes
                .values()
                .filter(|n| !n.tombstone)
                .map(|n| n.connections.len() as isize - 1)
                .max()
                .unwrap_or(-1);
        }
    }

    /// Graph shape and approximate memory use.
    pub fn stats(&self) -> HnswStats {
        let mut total_nodes = 0;
        let mut tombstones = 0;
        let mut layer_sizes = vec![0; (self.max_level + 1) as usize];
        let mut total_connections = 0;

        for node in self.nodes.values() {
            if node.tombstone {
                tombstones += 1;
            } else {
                total_nodes += 1;
                for size in layer_sizes.iter_mut().take(node.connections.len()) {
                    *size += 1;
                }
                total_connections += node.connections.first().map_or(0, Vec::len);
            }
        }

        let memory_usage_bytes = self.nodes.len() * std::mem::size_of::<(Uuid, HnswNode)>()
            + self
                .nodes
                .values()
                .map(|n| {
                    n.connections
                        .iter()
                        .map(|c| c.len() * std::mem::size_of::<Uuid>())
                        .sum::<usize>()
                })
                .sum::<usize>();

        HnswStats {
            total_nodes,
            tombstones,
            max_layer: self.max_level,
            layer_sizes,
            memory_usage_bytes,
            avg_connections: (total_nodes > 0)
                .then(|| total_connections as f32 / total_nodes as f32),
        }
    }

    /// Configured default for the search-time ef knob.
    pub fn get_ef_search(&self) -> usize {
        self.config.ef_search
    }
}

use crate::index::{IndexDetails, IndexSearchRequest, IndexStats, IndexType, VectorIndex};

// Resolves ef from the per-query config, or the index default.
impl VectorIndex for HnswIndex {
    fn insert(&mut self, id: Uuid, vector: &[f32], vectors: &dyn VectorReader) -> Result<()> {
        self.insert(id, vector, vectors)
    }

    fn search(&self, request: IndexSearchRequest<'_>) -> Result<Vec<Uuid>> {
        let ef = request
            .config
            .ef
            .unwrap_or_else(|| self.get_ef_search())
            .max(request.k);
        self.search(
            request.query,
            request.k,
            ef,
            request.vectors,
            request.filter,
            request.metadata,
        )
    }

    fn remove(&mut self, id: &Uuid) {
        self.remove(id);
    }

    fn stats(&self) -> IndexStats {
        let hnsw_stats = self.stats();

        IndexStats {
            index_type: IndexType::Hnsw,
            total_vectors: hnsw_stats.total_nodes,
            memory_usage_bytes: hnsw_stats.memory_usage_bytes,
            details: IndexDetails::Hnsw {
                max_layer: hnsw_stats.max_layer,
                layer_sizes: hnsw_stats.layer_sizes,
                avg_connections: hnsw_stats.avg_connections,
                ef_search: self.config.ef_search,
            },
        }
    }

    fn index_type(&self) -> IndexType {
        IndexType::Hnsw
    }

    fn metric(&self) -> piramid_hardware::compute::Metric {
        self.config.metric
    }

    fn set_execution(&mut self, mode: piramid_hardware::compute::ExecutionMode) {
        self.config.mode = mode;
    }

    fn build_config(&self) -> piramid_core::config::IndexConfig {
        piramid_core::config::IndexConfig::Hnsw {
            params: piramid_core::config::HnswConfig {
                mode: piramid_hardware::compute::ExecutionMode::default(),
                ..self.config
            },
        }
    }

    fn to_serializable(&self) -> crate::index::SerializableIndex {
        crate::index::SerializableIndex::Hnsw(self.clone())
    }
}
