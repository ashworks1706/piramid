# Indexing

A vector database without a good index is just a very sophisticated `for` loop. The core problem is this: given a query vector $\mathbf{q} \in \mathbb{R}^d$ and a collection of $N$ stored vectors, find the $k$ closest by some distance metric. The naïve approach scans every vector and computes the distance — $O(Nd)$ per query. At $N = 10^6$ with $d = 1536$ (OpenAI's `text-embedding-3-large`), that is 1.5 billion multiply-accumulate operations per query. Even with SIMD, you're looking at tens of milliseconds. That is fine for batch analysis; it is not fine for a latency-sensitive RAG pipeline that needs sub-millisecond retrieval.

Piramid supports three index types — Flat, IVF, and HNSW — with an Auto mode that picks the right one based on collection size. Each represents a different point on the exact recall / query latency tradeoff curve.

### Measuring quality: Recall@k

Before talking about how indexes work, it helps to be precise about what "good" means. The standard metric is **Recall@k**: the fraction of the true $k$ nearest neighbours that your ANN index actually returns.

$$\text{Recall@}k = \frac{|\text{ANN results} \cap \text{true } k\text{-NN}|}{k}$$

A Recall@10 of 0.95 means 9.5 out of 10 returned results are genuinely in the top-10 by exact distance — you missed half a result on average. For RAG, an LLM consuming the top-5 retrieved chunks rarely produces a noticeably worse answer if one chunk is an 11th-nearest rather than a 9th-nearest neighbour. The semantic delta is tiny. This is why targeting Recall@10 ≥ 0.95 is reasonable for production and you should not sacrifice 2–3× query latency chasing 0.99.

> **The recall-latency frontier** — every ANN algorithm traces a curve when you sweep its quality parameters. More recall always costs more latency. The goal of index design is to push that frontier as far to the upper-left (high recall, low latency) as possible. HNSW currently has the best-known frontier for general dense vectors at high dimensions.

### Why high-dimensional spaces break classical indexes

The reason Piramid doesn't use a B-tree or even a kd-tree for vector search is not laziness — it's that those structures provably collapse at high dimensionality. Understanding why is important context before looking at HNSW and IVF.

#### The kd-tree and why it fails

A kd-tree partitions $\mathbb{R}^d$ by recursively splitting on alternating axes. At $d = 2$ or $d = 3$ it is an elegant exact nearest-neighbour structure with $O(\log N)$ average query time. At $d = 1536$, it is slightly worse than a random scan.

The key quantity is the volume ratio of a hypersphere to its enclosing hypercube. In $d$ dimensions, the volume of a unit ball is:

$$V_d = \frac{\pi^{d/2}}{\Gamma(d/2 + 1)}$$

This goes to zero as $d \to \infty$. Practically, almost all the volume of a high-dimensional hypercube concentrates in its thin outer shell rather than its interior. An ANN query at radius $r$ around a point picks up essentially no volume — which means the kd-tree's bounding boxes contain almost no useful structure. The algorithm has to back up and explore essentially every branch, making the expected query complexity $O(2^d \cdot \log N)$ in the worst case, which at $d > 20$ is worse than brute force.

> **Intuition for the shell concentration:** In $d$ dimensions, the fraction of a unit ball's volume that lies within distance $\epsilon$ of the surface is $1 - (1-\epsilon)^d$. At $d = 1000$ and $\epsilon = 0.01$, this is $1 - 0.99^{1000} \approx 0.99996$ — essentially all of the ball is shell.

Ball trees, cover trees, and R-trees all suffer the same underlying problem. Any space-partitioning data structure that works by dividing $\mathbb{R}^d$ into regions will find that a query hypersphere at high $d$ intersects almost every region, destroying the logarithmic speedup.

#### Locality Sensitive Hashing

LSH takes a different approach. Rather than partitioning space, it probabilistically maps similar vectors to the same hash bucket, then only computes exact distances within a bucket. For a good hash family, the collision probability is a decreasing function of distance:

$$P(\text{hash}(\mathbf{x}) = \text{hash}(\mathbf{y})) = f\!\left(\frac{\|\mathbf{x} - \mathbf{y}\|}{\text{threshold}}\right)$$

For random hyperplane LSH (the standard choice for cosine similarity), the collision probability between two unit vectors is:

$$P(h(\mathbf{x}) = h(\mathbf{y})) = 1 - \frac{\theta}{\pi}, \quad \theta = \cos^{-1}(\mathbf{x} \cdot \mathbf{y})$$

To achieve recall $1 - \delta$ with $L$ independent hash tables and $K$ bits per table, you need:

$$L \geq \frac{\ln \delta}{\ln(1 - p_1^K)}$$

where $p_1$ is the collision probability at your target distance. This is workable but requires many hash tables (memory multiplier), and the constant factors are large. LSH dominated ANN benchmarks circa 2010–2014 but was comprehensively beaten in recall/latency/memory by HNSW once that paper appeared in 2016.

#### The curse of dimensionality, formally

The core issue underlying both failures is the concentration of measure. For $N$ vectors drawn uniformly from the unit hypercube in $\mathbb{R}^d$, the ratio of the maximum to minimum distance from a query point converges to 1:

$$\lim_{d \to \infty} \frac{\max_{x \in S} \|\mathbf{q} - \mathbf{x}\|_2 - \min_{x \in S} \|\mathbf{q} - \mathbf{x}\|_2}{\min_{x \in S} \|\mathbf{q} - \mathbf{x}\|_2} \to 0$$

When all distances are nearly equal, there is no local structure for a spatial index to exploit — every point is equidistant from every other. In practice at $d = 1536$ real embeddings are far from uniformly distributed (they cluster by topic and semantics), which is why ANN indexes still work. But the observation explains why cosine similarity outperforms raw Euclidean distance for text: normalising away magnitude removes one whole axis of spurious distance variation, making the distribution better-behaved.

> **What actually works at high dimensions:** graph-based indexes (HNSW) exploit the empirical cluster structure of the data directly rather than trying to partition abstract coordinates. Quantisation-based indexes (IVF + PQ) exploit the fact that even high-dimensional vectors often lie near a much lower-dimensional manifold. Neither tries to fight the geometry of $\mathbb{R}^d$ directly.

### Why approximate is good enough

Given that exact nearest neighbour is $O(Nd)$ and ANN indexes achieve ~97% recall at a fraction of the cost, the right question is whether that 3% miss rate matters for your application.

For RAG pipelines it almost never does. An LLM context window holds 5–20 retrieved chunks. If one of them is a 12th-nearest-neighbour rather than the 10th, the generated answer is unchanged with very high probability — the semantic gap between the 10th and 12th most similar vectors is imperceptible to a language model. The accuracy loss from a well-configured HNSW index is typically 1–5%, and the query latency improvement over exact search is 10–100×.

Where recall does matter: deduplication (you need exact matches, not approximate ones), compliance retrieval (must surface every relevant document), and similarity thresholding (returning "no match" for queries below a distance cutoff requires precise distances). For those workloads, Flat or a post-search exact reranker is appropriate.

### Flat — the honest baseline

The Flat index does not pretend to be smart. For every query it iterates over every known vector ID, computes the configured similarity score, then returns the top $k$ in $O(Nd)$ time. There is no build phase, no build memory overhead, and recall is exactly 1.0 by definition.

```rust
pub struct FlatIndex {
    config: FlatConfig,
    vector_ids: Vec<Uuid>,
}
```

That's the entire structure. The actual vectors live in the storage layer's HashMap; the flat index only holds the ordered list of IDs it has seen. The insert path is $O(1)$ amortised (vector append), the remove path is $O(N)$ (linear retain), and search is $O(Nd)$.

#### Why flat is fast at small N

Modern CPUs can execute SIMD dot products over `f32` arrays at roughly 16 multiply-accumulates per clock cycle on AVX2 (8 `f32` lanes × 2 fused multiply-add ports). At $d = 1536$ and 3.5 GHz, a single dot product takes on the order of:

$$t_{\text{dot}} \approx \frac{1536 \text{ ops}}{16 \text{ ops/cycle} \times 3.5 \times 10^9 \text{ Hz}} \approx 27\text{ ns}$$

Ten thousand such products take about 270 µs — squarely in the budget for a 1 ms response time. The entire dataset at $N = 10,000$ and $d = 1536$ occupies $10,000 \times 1536 \times 4 = 61\text{ MB}$, which fits comfortably in L3 cache on modern server CPUs (typically 32–96 MB). Once the working set is cache-warm, the scan is purely compute-bound rather than memory-bound, and the theoretical throughput is close to SIMD-peak.

> **SIMD warm cache vs cold:** the first scan after startup is memory-bound, reading 61 MB from RAM (~40 GB/s) in roughly 1.5 ms. Subsequent scans of the same warmed cache hit at L3 bandwidth (~300 GB/s) and finish in ~200 µs. Piramid's warming phase at startup (described in the storage post) is partly motivated by this: you want the first real user query to hit warm memory, not cold RAM.

The crossover point where flat becomes clearly slower than an ANN index depends on both $N$ and $d$. At $d = 128$, the flat scan stays competitive up to ~100K vectors because the vectors are smaller, the cache fits more of them, and the dot products are cheaper. At $d = 3072$ (some multimodal embeddings), the crossover arrives earlier — the 4× larger vectors mean 4× less fit in cache and 4× more compute per scan.

Piramid's auto-selector threshold of 10,000 is deliberately conservative: it leaves headroom for the case where the server's L3 is shared with other workloads.

### HNSW — navigating a small world

HNSW (Hierarchical Navigable Small World, Malkov and Yashunin 2018) is the algorithm behind most high-performance vector databases — Pinecone, Weaviate, Milvus, Qdrant, and Piramid all use it at their core. The intuition comes from graph theory's small-world phenomenon: in certain natural and engineered networks, the average shortest path between any two nodes grows only as $O(\log N)$ even as $N$ becomes very large. HNSW constructs exactly this kind of network over your vectors and traverses it greedily during search.

#### The small-world graph idea

The NSW (non-hierarchical) paper by Malkov et al. (2014) showed that if you build a graph where each vector-node is connected to its approximate nearest neighbours — plus a few "long-range" links to far-away nodes that act like highways — then greedy search on that graph finds nearest neighbours in $O(\log N)$ hops with good probability.

Each node starts with short-range connections to similar vectors, forming local clusters. The long-range edges emerge naturally from the construction order: vectors inserted early (when the graph was sparse) connect to whatever was available at the time, spanning large distances. Late insertions find dense local neighbourhoods. The mix produces the small-world property.

Greedy search simply starts from an arbitrary entry node and repeatedly moves to whichever current node's neighbour is closest to the query. It terminates when no neighbour is closer than the current node — a local minimum that, empirically, is almost always the global minimum for typical embedding distributions.

The raw NSW problem is that the entry point matters. Starting in the wrong region wastes many hops just navigating to the relevant part of the space. HNSW solves this with a hierarchy of layers: layer 0 is the dense local graph, and each higher layer is a progressively sparser "skip list in graph form" that lets you coarsely navigate to the right region before descending into the dense layer.

#### Layer probability and occupancy

Every newly inserted vector is assigned a maximum layer $\ell_{\max}$ drawn from a geometric distribution:

$$\ell_{\max} = \lfloor -\ln(u) \cdot m_L \rfloor, \quad u \sim \text{Uniform}(0,1), \quad m_L = \frac{1}{\ln M}$$

This gives $P(\ell_{\max} \geq \ell) = e^{-\ell / m_L} = e^{-\ell \ln M} = M^{-\ell}$. In other words, about $1/M$ of all vectors reach layer 1, $1/M^2$ reach layer 2, and so forth. The expected number of nodes at layer $\ell$ or above is:

$$\mathbb{E}[\text{nodes at layer} \geq \ell] = \frac{N}{M^\ell}$$

With $M = 16$ and $N = 1,000,000$: layer 0 has all 1M nodes, layer 1 has ~62,500, layer 2 has ~3,906, layer 3 has ~244. The total across all layers is:

$$\sum_{\ell=0}^{\infty} \frac{N}{M^\ell} = N \cdot \frac{1}{1 - 1/M} = N \cdot \frac{M}{M-1} \approx 1.067 N$$

So the multi-layer structure adds only about 6.7% overhead over storing the base layer alone. The hierarchy is nearly free in terms of node count.

```rust
fn random_layer(&self) -> usize {
    let r: f32 = rand::random();
    (-r.ln() * self.config.ml).floor() as usize
}
```

The default Piramid config is `m = 16`, `m_max = 32` (layer 0 gets $2 \times M$ connections because it needs to be denser to handle the full $N$ nodes), `ef_construction = 200`.

#### Inserting a vector — the two-phase algorithm

Insert is a top-down descent followed by a bottom-up connection phase.

**Phase 1 — find the entry region.** Starting from the global entry point (the node with the highest $\ell_{\max}$, which governs the top layer), the algorithm greedily descends from the top layer to $\ell_{\max}(\text{new node}) + 1$. At each layer in this descent, it performs a greedy 1-nearest-neighbour search, just to find which part of the space to descend into. At the end of phase 1 you have a candidate entry point at the new node's target layer that is already in the right neighbourhood.

**Phase 2 — connect at each layer.** From layer $\ell_{\max}$ down to layer 0, the algorithm runs a richer `ef_construction`-nearest-neighbour search, maintaining a candidate priority queue of the best `ef_construction` results seen so far. From those candidates it selects the best $M$ (or $M_{\max}$ at layer 0) and adds bidirectional edges:

```rust
for lc in (0..=layer).rev() {
    current_entry = self.search_layer(vector, &current_entry,
                                      self.config.ef_construction, lc, ...);
    let m = if lc == 0 { self.config.m_max } else { self.config.m };
    let neighbors = self.select_neighbors(&current_entry, m, vectors, vector);
    for &neighbor_id in &neighbors {
        // add new_node → neighbor edge
        pending_connections[lc].push(neighbor_id);
        // add neighbor → new_node edge (bidirectional)
        neighbor.connections[lc].push(id);
        // prune neighbor's list back to m if it exceeded
        if neighbor.connections[lc].len() > m {
            neighbor.connections[lc] = self.select_neighbors(..., m, ...);
        }
    }
}
```

The bidirectionality is critical. A unidirectional graph would have large "dead end" regions — nodes that many other nodes point to but that point back to nothing useful. Bidirectional edges guarantee that traversal can always go in both directions, which is what makes the small-world property hold under greedy descent.

After adding the back-edge, if the neighbour's connection list exceeds $M$, it gets pruned back. Piramid uses simple distance-based selection: keep the $M$ closest. The original HNSW paper proposes a diversity-aware heuristic that prefers candidates spread across different directions, which improves recall slightly but adds implementation complexity. Piramid's simple greedy selection works well in practice.

Time complexity per insert is $O(M \cdot ef_{\text{const}} \cdot \log N)$ — the $\log N$ comes from the number of layers, $M$ from the connections per layer, and $ef_{\text{const}}$ from the width of each layer's search.

#### The beam search — formal invariant

The core of both insert and query is `search_layer`, which runs on a single layer. Understanding it precisely is important because `ef` is the main tuning knob exposed to users.

The algorithm maintains two data structures simultaneously:

- A **candidates** max-heap ordered by *negative* distance (so the top element is the closest candidate yet explored)
- A **nearest** max-heap ordered by distance (so the top element is the *furthest* among the current best results)

The invariant at each step: `nearest` holds the `ef` best nodes seen so far; `candidates` holds nodes whose neighbours haven't been explored yet that might still improve the result. The loop runs while there exist unexplored candidates closer to the query than the current furthest result:

```rust
while let Some(candidate) = candidates.pop() {
    if candidate.distance > furthest_distance { break; }
    for &neighbor_id in &node.connections[level] {
        if visited.insert(neighbor_id) {
            let dist = self.distance(query, neighbor_vector);
            if dist < furthest_distance || nearest.len() < num_closest {
                candidates.push(neighbor);
                nearest.push(neighbor);
                if nearest.len() > num_closest { nearest.pop(); }
                furthest_distance = nearest.peek().map(|c| c.distance)...;
            }
        }
    }
}
```

The termination condition `candidate.distance > furthest_distance` is key: once the closest unexplored candidate is already farther than our worst current result, expanding it can only add worse results, so we stop. This makes the algorithm output-sensitive: dense clusters terminate quickly (they fill `nearest` fast and tighten `furthest_distance` fast), while sparse regions iterate longer.

#### ef_construction vs ef_search

These two parameters are often confused because they're both called `ef` internally but serve entirely different purposes.

`ef_construction` controls graph quality at build time. It is the beam width used during `search_layer` while inserting — higher values mean each new node finds better neighbours, resulting in a denser and more accurate graph. The cost is $O(ef_{\text{const}})$ per layer per insert. Setting `ef_construction = 200` means each insertion explores up to 200 candidates to find the best $M = 16$ neighbours to connect to.

`ef_search` controls query recall at query time. It is the beam width for the final layer-0 search. It does not affect the graph structure at all — only how thoroughly the already-built graph is explored during a query. You can set `ef_search = 50` at query time on a graph built with `ef_construction = 400` without any degradation to the graph itself.

> **The practical implication:** you should build with high `ef_construction` (200–400) and then tune `ef_search` per use case. A graph built with `ef_construction = 64` and queried with `ef_search = 400` will never reach the recall of a graph built with `ef_construction = 400` and queried with `ef_search = 200`, because the underlying graph simply doesn't have the connections needed. Build quality is permanent; search quality is adjustable.

In Piramid, both default to 200. Override `ef_search` per query via `SearchConfig`:

```yaml
collection:
  index:
    type: Hnsw
    m: 16
    ef_construction: 200
    ef_search: 200  # tune this at query time
```

At these settings, empirical Recall@10 on typical text embedding distributions is 97–99%.

#### Memory footprint of the graph

Each node in the HNSW graph stores:
- One `Vec<Vec<Uuid>>` of connections across all its layers. A node at layer 0 only has $M_{\max} = 32$ connections; a node at layer 1 has $M = 16$ more; and so on.
- A `tombstone: bool` flag (1 byte).

The expected total storage for connections is:

$$\text{graph memory} = N \cdot M_{\max} \cdot \frac{M}{M-1} \cdot 16\text{ bytes per UUID}$$

With $N = 10^6$, $M = 16$, $M_{\max} = 32$: roughly $10^6 \times 32 \times (16/15) \times 16 \approx 546\text{ MB}$. That is on top of the raw vector storage ($10^6 \times 1536 \times 4 \approx 6.14\text{ GB}$). The graph overhead is about 9% of vector memory — a modest tax for the search speedup.

#### Tombstoning and graph connectivity

Deletion in graph-based indexes is notoriously awkward. Physically removing a node and its edges can disconnect parts of the graph — future traversals may fail to reach regions that were only accessible through the removed node. This is not a theoretical concern: if a hub node (one that happens to be the entry path to an entire cluster) is deleted and its edges removed, searches into that cluster will silently produce incorrect results.

HNSW in Piramid uses **tombstoning**: a deleted node has its `tombstone` flag set to `true` but its edges are kept intact in memory.

```rust
fn mark_tombstone(&mut self, id: &Uuid) {
    if let Some(node) = self.nodes.get_mut(id) {
        node.tombstone = true;
    }
}
```

During search, tombstoned nodes are used as traversal intermediaries — their edges are still followed when exploring the graph — but they are filtered out before the result set is returned. This guarantees graph connectivity at the cost of retaining dead nodes in memory.

> **Tombstone accumulation risk:** if a workload has high delete rates, tombstones pile up. The graph's "live density" — the number of non-tombstoned nodes per layer — gradually decreases, and eventually traversal is slow because a large fraction of the explored nodes are dead weight. The fix is a full index rebuild from the live vectors, which Piramid triggers through its compaction mechanism. After compaction, the loaded index is a clean graph with no tombstones.

### IVF — Voronoi cells and k-means

IVF (Inverted File Index) is a completely different philosophy. Rather than building a navigable graph, it partitions the vector space into $K$ clusters and builds an inverted list: a mapping from cluster ID to the set of vector IDs assigned to that cluster. At query time, instead of scanning all $N$ vectors, it only scans the `nprobe` closest clusters. If `nprobe = 1` and the clusters are well-balanced, the expected scan is $N / K$ vectors per query rather than $N$ — a $K$-fold speedup.

The geometry here is equivalent to **Voronoi diagrams**. Each centroid $\mathbf{c}_i$ defines a Voronoi cell $V_i = \{ \mathbf{x} : \|x - c_i\|_2 \leq \|x - c_j\|_2 \;\forall j \neq i \}$. A query vector $\mathbf{q}$ falls into the cell whose centroid is nearest, and searching `nprobe` clusters means searching that cell plus its $(\text{nprobe} - 1)$ nearest neighbours. Results near a cell boundary may still be missed — that's the fundamental recall limitation of IVF.

#### Building the clusters

Cluster centroids are computed offline using Lloyd's k-means algorithm. Starting from $K$ randomly selected vectors as initial centroids:

1. Assign each vector to its nearest centroid: $c(x) = \arg\min_i \|\mathbf{x} - \mathbf{c}_i\|$
2. Recompute centroid $i$ as the mean of its assigned vectors: $\mathbf{c}_i \leftarrow \frac{1}{|V_i|} \sum_{x \in V_i} \mathbf{x}$
3. Repeat until convergence or `max_iterations` is hit.

Piramid defaults to `max_iterations = 10`, which is light but fast. For most distributions, k-means converges in a handful of passes — the later iterations produce diminishing improvements.

The right value of $K$ depends on $N$. The standard heuristic is $K \approx \sqrt{N}$, which balances cluster granularity against inverted list scan cost. Piramid's auto-config implements this directly:

```rust
pub fn auto(num_vectors: usize) -> Self {
    let num_clusters = (num_vectors as f32).sqrt().max(10.0) as usize;
    let num_probes = (num_clusters as f32 * 0.1).max(1.0).min(10.0) as usize;
    ...
}
```

So at $N = 100,000$, you get $K = 316$ clusters and `num_probes = 10`. Each cluster holds roughly 316 vectors on average, and a query scans $10 \times 316 = 3,160$ vectors instead of 100,000. That's a 32x speedup with decent recall because probing 10/316 ≈ 3% of the space gives good coverage of the local neighbourhood.

#### Online insertion and the cluster drift problem

When new vectors arrive after the clusters are built, IVF assigns them to whichever existing centroid is nearest and appends them to that inverted list — no re-clustering. This is fast and correct as long as the new data looks like the training data. If the distribution shifts (say, you start indexing a completely different topic), some clusters become overloaded and others become sparse, degrading both recall and the $\sqrt{N}$ search complexity. The fix is periodic re-clustering, which is triggered by Piramid's compaction / rebuild flow.

#### Searching with nprobe

At query time, IVF first identifies the `nprobe` closest centroids by scanning all $K$ of them (fast, since $K \ll N$). Then it scans the inverted lists for those clusters, scores every vector in them, and returns the top $k$. The `nprobe` parameter lets you trade off recall for speed:

| nprobe | recall (approx) | vectors scanned |
|--------|----------------|-----------------|
| 1 | ~70% | $N/K$ |
| 5 | ~90% | $5N/K$ |
| 10 | ~95% | $10N/K$ |
| $K$ | 100% | $N$ (flat scan) |

Setting `nprobe = K` degrades IVF exactly to the Flat index, which is a useful debugging check.

### Auto-selection

Piramid's `IndexConfig::Auto` selects the index type based on collection size at build time:

```
< 10,000 vectors  →  Flat
10,000–100,000    →  IVF
> 100,000         →  HNSW
```

This mirrors the conventional wisdom in the ANN libraries community. Below 10K, the flat scan fits in L3 cache and often beats any ANN index on raw latency while giving perfect recall. The IVF middle range is where the graph structure of HNSW would be overloaded during construction (you'd spend as many operations building as you'd save searching), but where brute force is getting noticeably slow. Above 100K, HNSW's $O(\log N)$ search and strong recall make it the clear winner for a latency-first system.

These thresholds are tunable if you specify the index type explicitly in your collection config. If you need HNSW at 5,000 vectors because your query rate is very high, nothing stops you.

### Search parameters

Three parameters shape the quality / speed tradeoff at query time, and they surface in the `SearchConfig` struct.

**`ef`** is the beam width for HNSW layer-0 search. It is the number of candidates the algorithm maintains in its priority queue before committing to the final top $k$. Setting `ef < k` is illegal (Piramid clamps it to `max(ef, k)`), and setting `ef = k` gives the minimum possible work — essentially just keep the first $k$ candidates found. Higher values explore more of the graph neighbourhood. `ef = 200` is a solid general-purpose setting; `ef = 400` is high-recall; `ef = 50` is for bulk ingestion latency tests where recall doesn't matter.

**`nprobe`** is the equivalent knob for IVF. Higher values probe more clusters, increasing recall and scan work proportionally.

**`filter_overfetch`** is a multiplier applied when a metadata filter is present. Because HNSW and IVF can only apply filters during neighbour iteration — not before — at query time Piramid fetches $k \times \text{filter\_overfetch}$ candidates from the index, applies the filter to that expanded set, and then truncates to $k$. The default is 10. If your filter is highly selective (say, matching only 2% of vectors), you'd want to raise this significantly, otherwise you'll consistently get fewer than $k$ results after the filter pass.

```rust
let search_k = if params.filter.is_some() {
    k.saturating_mul(expansion)
} else {
    k
};
```

The three presets in `SearchConfig` are `fast()` (`ef = 50`, `nprobe = 1`), `balanced()` (defaults), and `high()` (`ef = 400`, `nprobe = 20`), which cover most use cases without manual tuning.

### Insert, update, and remove paths

All three index types implement the same `VectorIndex` trait:

```rust
pub trait VectorIndex: Send + Sync {
    fn insert(&mut self, id: Uuid, vector: &[f32], vectors: &HashMap<Uuid, Vec<f32>>);
    fn search(&self, ...) -> Vec<Uuid>;
    fn remove(&mut self, id: &Uuid);
    fn stats(&self) -> IndexStats;
    fn index_type(&self) -> IndexType;
}
```

The signature of `insert` passes the entire `vectors` map even though Flat and IVF don't need it. This is because HNSW needs to compute distances to existing nodes during graph construction — it can't just insert in isolation. The design keeps the trait simple at the cost of passing a reference to potentially large memory.

Update is not a first-class operation. An update to an existing vector is handled at the storage layer as a remove followed by an insert. This is the correct semantic: the old vector occupies a position in the graph or cluster, and repositioning it requires removing and re-inserting with its new embedding.

For HNSW, `remove` tombstones the node. For IVF, `remove` looks up the vector's cluster in the `vector_to_cluster` map (`O(1)`) and removes it from the inverted list (`O(cluster_size)` in the worst case). For Flat, `remove` is `O(N)` since it's a linear scan through the ID list.

The vector index is always kept in memory and is the source of truth for fast queries. Disk persistence of the index happens as part of the compaction cycle through the storage layer's serialisation path — the index structs derive `serde::Serialize` and `serde::Deserialize`, so they can round-trip cleanly to the `.vidx.db` file.

### Product quantisation and future compression

Product quantisation (PQ) is the standard technique for reducing vector memory by a factor of 8–32× with minimal recall degradation. The idea is to split a $d$-dimensional vector into $m$ subvectors of $d/m$ dimensions each, train a small codebook of 256 centroids for each subspace, and replace each subvector with its 8-bit centroid ID. A 1536-dimensional float32 vector (6KB) becomes 192 bytes. Distance approximations use precomputed lookup tables rather than the full inner product, making SIMD-accelerated approximate distance cheap.

Piramid's quantisation module (`src/quantisation/`) exists in scaffolded form and is on the roadmap. Once integrated, PQ codes would be stored alongside the graph structure, and the inner loop of `search_layer` would consult the lookup table instead of computing exact distances — dropping the per-hop cost from $O(d)$ to $O(m)$ multiply-accumulates. The reranking pass (computing exact distances on the final `ef` candidates) would still use full vectors, keeping recall high while dramatically reducing the search-phase compute.
