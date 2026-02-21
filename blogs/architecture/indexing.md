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

#### [Locality Sensitive Hashing](https://en.wikipedia.org/wiki/Locality-sensitive_hashing)

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

![Curse of dimensionality — as dimensions grow, all pairwise distances concentrate around the same value and spatial indexes lose their ability to prune the search space](https://towardsdatascience.com/wp-content/uploads/2023/12/1BSCbxVtV4F6dCkcAL1-y-A.png)

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

[HNSW (Hierarchical Navigable Small World, Malkov and Yashunin 2018)](https://arxiv.org/abs/1603.09320) is the algorithm behind most high-performance vector databases — [Pinecone](https://www.pinecone.io/), Weaviate, Milvus, Qdrant, and Piramid all use it at their core. The intuition comes from graph theory's small-world phenomenon: in certain natural and engineered networks, the average shortest path between any two nodes grows only as $O(\log N)$ even as $N$ becomes very large. HNSW constructs exactly this kind of network over your vectors and traverses it greedily during search.

![HNSW layered graph — layer 2 is sparse for long-range navigation, layer 1 is denser, layer 0 holds all nodes with the full connection density; search descends from top to bottom](https://miro.medium.com/1*hEu_9Z1Ra5ndhDS1n_Kjdg.png)

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

![IVF vs HNSW](https://encrypted-tbn0.gstatic.com/images?q=tbn:ANd9GcQm1J-FSFmhU3OB-AdeufzY3Msu5Bmk1iNbCQ&s)

IVF (Inverted File Index) takes a completely different approach to the ANN problem. Rather than building a navigable graph, it partitions the vector space into $K$ clusters using [k-means](https://en.wikipedia.org/wiki/K-means_clustering), and for each cluster maintains an **inverted list** — a list of vector IDs assigned to that cluster. At query time it only scans the `nprobe` closest clusters instead of all $N$ vectors.

The geometric picture is a **[Voronoi diagram](https://en.wikipedia.org/wiki/Voronoi_diagram)**. Each centroid $\mathbf{c}_i$ defines a cell:

$$V_i = \bigl\{ \mathbf{x} \in \mathbb{R}^d : \|\mathbf{x} - \mathbf{c}_i\| \leq \|\mathbf{x} - \mathbf{c}_j\| \;\forall j \neq i \bigr\}$$

A query vector $\mathbf{q}$ falls into the cell whose centroid is nearest. Probing `nprobe` clusters means searching the query's own Voronoi cell plus the $(\text{nprobe} - 1)$ adjacent cells — the ones whose centroids are next-closest to $\mathbf{q}$. Any true nearest neighbour that lies near a cell boundary may exist in an adjacent cell, which is the fundamental recall limitation of IVF with small `nprobe`.

#### Building the clusters — [Lloyd's algorithm](https://en.wikipedia.org/wiki/Lloyd%27s_algorithm)

Centroid computation uses Lloyd's k-means algorithm:

1. **Initialise:** pick $K$ vectors at random as initial centroids $\mathbf{c}_1, \ldots, \mathbf{c}_K$.
2. **Assign:** for each vector $\mathbf{x}_i$, find its nearest centroid: $c(\mathbf{x}_i) = \arg\min_j \|\mathbf{x}_i - \mathbf{c}_j\|$.
3. **Update:** recompute each centroid as the mean of its assigned vectors: $\mathbf{c}_j \leftarrow \frac{1}{|V_j|} \sum_{\mathbf{x} \in V_j} \mathbf{x}$.
4. Repeat steps 2–3 until convergence or `max_iterations`.

Piramid runs 10 iterations by default. Lloyd's algorithm minimises the within-cluster sum of squared distances (inertia):

$$\mathcal{J} = \sum_{j=1}^{K} \sum_{\mathbf{x} \in V_j} \|\mathbf{x} - \mathbf{c}_j\|^2$$

Each iteration monotonically decreases $\mathcal{J}$, so convergence is guaranteed. In practice the large gains come in the first 3–5 iterations; later iterations move centroids by tiny amounts. Ten iterations is a reasonable engineering tradeoff between cluster quality and startup cost.

> **k-means++ vs random initialisation:** Piramid uses random initialisation (just takes the first $K$ vectors). [k-means++](https://en.wikipedia.org/wiki/K-means%2B%2B) initialises centroids by sampling proportionally to $\|\mathbf{x} - \text{nearest existing centroid}\|^2$, which produces better initial spread and usually converges in fewer iterations. It's a potential improvement to the build phase for distributions where random initialisation produces early clustering near dense regions.

#### Why K ≈ √N is optimal

The total cost of an IVF query is:

Each iteration monotonically decreases $\mathcal{J}$, so convergence is guaranteed. In practice the large gains come in the first 3–5 iterations; later iterations move centroids by tiny amounts. Ten iterations is a reasonable engineering tradeoff between cluster quality and startup cost.

> **k-means++ vs random initialisation:** Piramid uses random initialisation (just takes the first $K$ vectors). [k-means++](https://en.wikipedia.org/wiki/K-means%2B%2B) initialises centroids by sampling proportionally to $\|\mathbf{x} - \text{nearest existing centroid}\|^2$, which produces better initial spread and usually converges in fewer iterations. It's a potential improvement to the build phase for distributions where random initialisation produces early clustering near dense regions.

#### Why K ≈ √N is optimal

The total cost of an IVF query is:

$$\text{cost} = \underbrace{\alpha \cdot K}_{\text{centroid scan}} + \underbrace{\beta \cdot \frac{N \cdot \text{nprobe}}{K}}_{\text{inverted list scan}}$$

where $\alpha$ is the cost per centroid distance computation and $\beta$ is the cost per vector distance computation inside the inverted list. To minimise over $K$ (treating nprobe as fixed at 1), take the derivative and set to zero:

$$\frac{\partial \text{cost}}{\partial K} = \alpha - \beta \cdot \frac{N}{K^2} = 0 \implies K^* = \sqrt{\frac{\beta N}{\alpha}}$$

Since $\beta \approx \alpha$ (both are dot products of similar-dimensional vectors with the same SIMD path), we get $K^* \approx \sqrt{N}$. This is where Piramid's auto-config comes from:

```rust
pub fn auto(num_vectors: usize) -> Self {
    let num_clusters = (num_vectors as f32).sqrt().max(10.0) as usize;
    let num_probes = (num_clusters as f32 * 0.1).max(1.0).min(10.0) as usize;
    ...
}
```

At $N = 100,000$: $K = 316$, `num_probes = 10`. Average cluster size is $N/K = 316$ vectors. A query scans $10 \times 316 = 3,160$ vectors — a 32× reduction over the full scan, with recall typically around 95%.

At this optimal $K$, the total query cost is $2\alpha \sqrt{N}$ (verify: $\alpha K + \alpha N / K = \alpha \sqrt{N} + \alpha \sqrt{N}$). Compare to flat scan at $\alpha N$: IVF is $\sqrt{N} / (2\alpha) \cdot \alpha = 1/(2\sqrt{N})$ times the flat cost, so the speedup factor scales as $\sqrt{N}/2$. At $N = 10^6$, that's a theoretical 500× speedup over flat — in practice you get 50–100× because of cache effects and overhead.

#### The cluster boundary problem

The recall degradation with small `nprobe` comes from vectors that land near cell boundaries. If your nearest neighbour and your query are separated by a Voronoi boundary, the true nearest neighbour is in a different cluster than the one containing the query. With `nprobe = 1`, it is missed.

How common is this? For uniformly distributed points, the expected fraction of a dataset that lies near any boundary scales roughly as $1/K^{1/d}$ — essentially 100% at high dimensions, because every vector is near some boundary (again, concentration of measure). For real embedding distributions which are manifold-like rather than uniform, the fraction is much smaller because the data has strong cluster structure.

This is why IVF works poorly for synthetic uniform distributions in benchmarks but works well on real embedding datasets: semantic structure means vectors genuinely separate into clusters, and boundaries are relatively sparse in the data-dense regions.

#### Online insertion and cluster drift

After the clusters are built, new vectors are assigned to the nearest existing centroid and appended to that inverted list. No re-clustering happens. This is correct for stationary data distributions, but if the distribution shifts over time — new document topics, new languages, new embedding model — some clusters become overloaded (recall degrades because the list is too long) and others become nearly empty (throughput degrades because you're scanning small irrelevant lists). The `nprobe` budget buys you decreasing recall per probe when clusters are imbalanced.

The fix is a periodic rebuild: re-run k-means on the current live vector set, reassign all vectors to new centroids, and reconstruct the inverted lists. Piramid triggers this through the compaction mechanism exactly as HNSW triggers a graph rebuild.

#### Searching with nprobe

At query time, IVF scans all $K$ centroids to find the `nprobe` nearest (this is a flat scan over centroids — fast, since $K \ll N$). Then it scores every vector in those `nprobe` inverted lists and returns the top $k$. The IVF code falls back to a brute-force scan if clusters haven't been built yet:

```rust
// Find nearest centroids
centroid_distances.sort_by(|a, b| b.1.partial_cmp(&a.1)...);
let nprobe = quality.nprobe.unwrap_or(self.config.num_probes);
for (cluster_id, _) in centroid_distances.iter().take(nprobe) {
    for id in &self.inverted_lists[*cluster_id] {
        let score = self.config.metric.calculate(query, vec, self.config.mode);
        candidates.push((*id, score));
    }
}
```

| nprobe | recall (approx) | vectors scanned |
|--------|----------------|-----------------|
| 1 | ~70% | $N/K$ |
| 5 | ~90% | $5N/K$ |
| 10 | ~95% | $10N/K$ |
| $K$ | 100% | $N$ (flat scan) |

Setting `nprobe = K` degrades IVF exactly to the Flat index — a useful debugging check when something seems wrong with recall.

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

Three parameters shape the quality / speed tradeoff at query time, all surfaced through `SearchConfig`.

**`ef`** is the beam width for HNSW layer-0 search. It controls how many candidate nodes the algorithm holds in its result heap before committing to the top $k$. Setting `ef < k` is illegal (Piramid clamps to `max(ef, k)`). Setting `ef = k` gives minimum search work. The empirical recall curve is roughly:

| ef | Recall@10 (approximate) | latency multiplier |
|----|------------------------|--------------------|
| 10 | ~85% | 1× |
| 50 | ~93% | 1.8× |
| 100 | ~96% | 2.5× |
| 200 | ~98% | 4× |
| 400 | ~99.5% | 7× |

These numbers vary with dataset distribution and $M$, but the general shape is consistent: recall gains become logarithmically more expensive as you push toward 100%.

**`nprobe`** is the equivalent knob for IVF — the number of clusters to search.

**`filter_overfetch`** deserves a careful explanation because filtered vector search is one of the harder practical problems in the space.

#### Pre-filter, post-filter, and in-traversal filter

There are three strategies for combining metadata filters with ANN search:

**Pre-filter:** build a separate inverted index over your metadata, run the filter first to get a candidate set, then run ANN search restricted to that set. This gives exact-filter recall but requires either a separate index structure or re-querying the ANN index with a custom candidate set — complex to implement and expensive if the filter matches many documents.

**Post-filter (naïve):** run ANN search for $k$ results, then apply the filter. If the filter rejects most results, you get fewer than $k$ results back, possibly zero. This is the simplest approach and works fine when the filter is loose.

**In-traversal filter (Piramid's approach):** pass the filter into the HNSW `search_layer` loop, so that filtered-out nodes contribute to graph traversal (they're still used as stepping stones) but are excluded from the result heap. This is better than post-filter because the traversal continues until `ef` *qualifying* candidates are found rather than stopping at `ef` total candidates. The implementation in `search_layer` checks each neighbour against the filter before adding it to `nearest`:

```rust
if let Some(f) = filter {
    if let Some(md) = metadatas.get(&neighbor_id) {
        if !f.matches(md) { continue; }  // skip result, but still traverse edges
    }
}
// ... add to nearest and candidates
```

The problem with in-traversal filtering is that it doesn't help with **sparse filters** — filters that match only 1–5% of the dataset. If only 1 in 100 vectors is eligible, an `ef = 200` beam search may exhaust its entire candidate budget before finding 10 qualifying results. You always get at most `ef / selectivity` useful results from a single pass.

The `filter_overfetch` parameter addresses this by inflating the request to the index:

$$k_\text{search} = k \times \text{filter\_overfetch}$$

```rust
let search_k = if params.filter.is_some() {
    k.saturating_mul(expansion)
} else { k };
```

With the default `filter_overfetch = 10` and `k = 5`, the index fetches 50 candidates before filtering. If the filter has 10% selectivity, you expect 5 qualifying results on average. For 2% selectivity you'd want `filter_overfetch = 50`. The tradeoff is straightforward: overfetch linearly increases both the number of distance computations and the chance of finding enough qualified results.

> **When overfetch isn't enough:** for very selective filters (< 1% of vectors), even large overfetch values fail because the ANN graph simply can't surface enough candidates from a small eligible set in a single traversal. The right solution for highly selective filters is a purpose-built filtered index (sometimes called "filtered HNSW" or "attribute-aware HNSW") that maintains separate entry points per filter category. This is on Piramid's roadmap.

The three `SearchConfig` presets are:
- `fast()` — `ef = 50`, `nprobe = 1` — for batch pipelines where high throughput matters more than last-mile recall
- `balanced()` — defaults (ef = config value, nprobe = config value) — appropriate for most interactive RAG
- `high()` — `ef = 400`, `nprobe = 20` — for compliance retrieval or precision-critical tasks

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

As collections scale past a few million vectors, memory becomes the binding constraint. A 1536-dimensional `f32` embedding is 6,144 bytes. Ten million of them occupy ~58 GB — well beyond single-server RAM. Product quantisation (PQ) is the standard technique for compressing vectors by 8–64× with only modest recall degradation.

#### How PQ works

PQ splits the $d$-dimensional vector into $m$ disjoint subvectors of $d/m$ dimensions each:

$$\mathbf{x} = [\mathbf{x}^{(1)}, \mathbf{x}^{(2)}, \ldots, \mathbf{x}^{(m)}], \quad \mathbf{x}^{(j)} \in \mathbb{R}^{d/m}$$

For each subspace $j$, it trains a separate k-means codebook with 256 centroids (fitting in one byte):

$$C_j = \{\mathbf{c}_{j,0}, \mathbf{c}_{j,1}, \ldots, \mathbf{c}_{j,255}\} \subset \mathbb{R}^{d/m}$$

Each vector is then encoded as a sequence of $m$ byte-sized centroid IDs:

$$\text{code}(\mathbf{x}) = \bigl[\arg\min_{i} \|\mathbf{x}^{(1)} - \mathbf{c}_{1,i}\|, \;\ldots,\; \arg\min_{i} \|\mathbf{x}^{(m)} - \mathbf{c}_{m,i}\|\bigr] \in \{0,\ldots,255\}^m$$

For $d = 1536$ and $m = 192$: each subvector has $1536/192 = 8$ dimensions, the code is 192 bytes. Compression ratio: $1536 \times 4 / 192 = 32\times$.

#### Approximate distance via lookup tables

The power of PQ is not just compression — it enables fast approximate distance computation. Given a query $\mathbf{q}$, precompute a distance table $T \in \mathbb{R}^{m \times 256}$:

$$T[j][i] = \|\mathbf{q}^{(j)} - \mathbf{c}_{j,i}\|^2$$

This table has $m \times 256$ entries and costs $O(m \cdot 256 \cdot d/m) = O(256d)$ to build — a one-time cost per query. Then the approximate squared distance between query $\mathbf{q}$ and any database vector $\mathbf{x}$ with code $c$ is:

$$\hat{d}^2(\mathbf{q}, \mathbf{x}) \approx \sum_{j=1}^{m} T[j][c_j(\mathbf{x})]$$

This is $m$ table lookups and additions — $O(m)$ rather than $O(d)$. At $m = 192$ vs $d = 1536$: **8× fewer operations per distance computation**, in addition to the 32× memory saving. The practical result on HNSW is that the inner loop of `search_layer` (which runs thousands of times per query) becomes massively cheaper.

> **Asymmetric Distance Computation (ADC):** the scheme above keeps the query in full precision and quantises only the database vectors. This is called ADC and is standard in practice — the query is free since there's only one of them. Symmetric quantisation (also compressing the query) is 32× cheaper still but loses recall rapidly.

#### Reranking preserves recall

The PQ distances are approximate. To recover precision, the standard pipeline is:

1. Run beam search / IVF with PQ distances — fast, low memory, moderate recall.
2. Take the top $\gamma \cdot k$ candidates ($\gamma \approx 3$–$10$).
3. Rerank by computing exact distances with the original float32 vectors for just those $\gamma k$ candidates.

The exact reranking step costs $O(\gamma k \cdot d)$ — cheap because $\gamma k \ll \text{ef}$. The combined recall of this pipeline approaches exact-search recall at a fraction of the memory and compute.

#### Piramid's roadmap

Piramid's `src/quantization/` module exists in scaffolded form. Once integrated, the PQ codes would be stored alongside the HNSW graph in the `.vidx.db` file, and `search_layer`'s distance function would use lookup-table ADC instead of full dot products. The reranking pass over the final `ef` candidates would still use mmap'd float32 vectors, keeping recall high while the search-phase compute drops by 8× and memory drops by 32×.

For a $10^6$-vector collection at $d = 1536$: current memory is ~6.14 GB for vectors + ~546 MB for the graph = 6.7 GB. With PQ ($m = 192$): ~192 MB for codes + same ~546 MB graph = 738 MB. That is a 9× total reduction, bringing large collections well within single-server RAM without a GPU.
