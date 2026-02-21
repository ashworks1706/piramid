# Query

In the [previous sections](/blogs/architecture/indexing) we looked at how vectors get inserted and organised into index structures. Now let's talk about the other side of that: what actually happens when you send a query. It sounds conceptually simple, but the mechanics of doing that at scale without brute-forcing every comparison are genuinely non-trivial, and the tradeoffs involved are worth understanding.

### What the problem actually is

You have a collection of $n$ vectors $\{\mathbf{x}_1, \mathbf{x}_2, \ldots, \mathbf{x}_n\} \subset \mathbb{R}^d$. You receive a query vector $\mathbf{q} \in \mathbb{R}^d$ and want the $k$ vectors from the collection that are most similar to $\mathbf{q}$ under some distance or similarity function $\text{sim}(\cdot, \cdot)$.

Formally, you want:

$$\text{kNN}(\mathbf{q}, k) = \underset{S \subseteq [n],\, |S|=k}{\arg\max} \sum_{i \in S} \text{sim}(\mathbf{q}, \mathbf{x}_i)$$

If you just compute this directly — calculate $\text{sim}(\mathbf{q}, \mathbf{x}_i)$ for every $i$, sort, and take the top $k$, which is $O(nd)$ operations per query. For $n = 10^6$ and $d = 1536$, that's $1.536 \times 10^9$ floating-point operations per query. A modern CPU doing 1 GFLOP/s (single-threaded for this kind of sequential scan) takes about 1.5 seconds. A single GPU can do it sub-100ms, but the machine still has to touch all $nd$ floats. At $n = 10^8$ (a reasonably sized production corpus) you're looking at minutes per query no matter how fast your hardware is.

The entire field of approximate nearest neighbor (ANN) search exists to escape this linear bottleneck.

---

### Distance metrics

Before getting into algorithms, it's worth being precise about what "similarity" means geometrically. There are three metrics that actually matter for practical embedding search:

**Cosine similarity** measures the angle between two vectors, ignoring their magnitudes:

$$\cos(\mathbf{a}, \mathbf{b}) = \frac{\mathbf{a} \cdot \mathbf{b}}{\|\mathbf{a}\|_2 \|\mathbf{b}\|_2}$$

Range is $[-1, 1]$, higher is more similar. This is the right choice for most embedding models since modern text embeddings are almost always $\ell_2$-normalised at the source, and cosine similarity of normalised vectors is equal to their dot product. It's also computationally equivalent to Euclidean distance on the unit sphere: $\|\mathbf{a} - \mathbf{b}\|^2 = 2 - 2\cos\theta$ when $\|\mathbf{a}\| = \|\mathbf{b}\| = 1$.

**Euclidean distance** is physical distance through the embedding space:

$$d_{\text{euc}}(\mathbf{a}, \mathbf{b}) = \sqrt{\sum_{i=1}^d (a_i - b_i)^2} = \|\mathbf{a} - \mathbf{b}\|_2$$

Smaller is more similar. You'd use this when the magnitude of embeddings carries meaningful information, like certain image embeddings, physics simulation vectors, or scenarios where you're representing a literal position in space rather than a direction.

**Dot product** is the unnormalised inner product:

$$\mathbf{a} \cdot \mathbf{b} = \sum_{i=1}^d a_i b_i$$

Larger is more similar. Useful for recommendation systems where vector magnitude encodes something meaningful (item popularity, user engagement level). A more popular item having a higher-magnitude embedding will naturally rank higher under dot product search, which can be the desired behaviour. Under $\ell_2$ normalisation, dot product and cosine similarity are identical. Without normalisation, they diverge.

Piramid's `Metric::calculate` does a small but important transform on Euclidean: it returns $1 / (1 + d_{\text{euc}})$ rather than the raw distance. This maps the unbounded $[0, \infty)$ range into $(0, 1]$ and makes it possible to sort and compare scores consistently across searches; a score of 0.95 means something similar whether the collection uses cosine or Euclidean. Dot product is returned raw since its scale depends entirely on the embedding model used.

---

### The curse of dimensionality

Tree-based spatial indexes like [KD-trees](https://en.wikipedia.org/wiki/K-d_tree), [ball-trees](https://en.wikipedia.org/wiki/Ball_tree), and [R-trees](https://en.wikipedia.org/wiki/R-tree) work well in 2, 3, maybe 10 dimensions. In $d = 1536$ they are essentially useless. Here's why.

In a KD-tree, a hyperplane splits space at each node. Searching for the nearest neighbor involves traversing the tree and backtracking whenever the current best candidate's hypersphere could intersect the other side of a split. In low dimensions, these backtracks are rare because the hyperspheres are tight. In high dimensions, the geometry breaks. The volume of a $d$-dimensional ball of radius $r$ is:

$$V_d(r) = \frac{\pi^{d/2}}{\Gamma(d/2 + 1)} r^d$$

As $d$ grows, all the volume of the ball concentrates near the *surface* (the shell), not in the interior. Equivalently, if you pick two random points on the unit hypersphere in $d$ dimensions, their distance converges to $\sqrt{2}$ with vanishing variance:

$$\mathbb{E}\left[\|\mathbf{x} - \mathbf{y}\|^2\right] = 2, \quad \text{Var}\left[\|\mathbf{x} - \mathbf{y}\|^2\right] \to 0 \text{ as } d \to \infty$$

This is the concentration of measure. When all pairwise distances concentrate around the same value, there's no meaningful structure for a tree to exploit, so you have to check nearly everything anyway. A KD-tree in $d = 100$ is already degraded to near-linear scan performance. At $d = 1536$ a tree structure adds overhead with essentially no benefit.

![Curse of dimensionality — as $d$ grows, the volume of a hypersphere shrinks relative to its enclosing cube and all points collapse to the same distance from any query, destroying the structure that spatial indexes rely on](https://cofactorgenomics.com/wp-content/uploads/2019/04/picture1.png)

---

### How ANN algorithms escape the linear trap

There are three broad families of approach.

**[Locality-sensitive hashing (LSH)](https://en.wikipedia.org/wiki/Locality-sensitive_hashing)** uses a family of hash functions $\{h_1, \ldots, h_L\}$ where similar vectors are likely to hash to the same bucket. For cosine similarity, the standard construction is random hyperplane hashing: for each hash function, sample a random vector $\mathbf{r}$ from a Gaussian, then $h(\mathbf{x}) = \text{sign}(\mathbf{r} \cdot \mathbf{x})$. Two vectors with angle $\theta$ between them collide in the same bucket with probability $1 - \theta/\pi$. You use $L$ independent hash tables and return the union of all matched buckets as candidates.

LSH has attractive theoretical guarantees but poor practical performance on modern high-dimensional dense embeddings. The number of hash tables needed to get good recall grows with dimension, and the bucket sizes are hard to tune: too coarse and you return too many false candidates, too fine and you miss true neighbors. In practice, graph-based methods have largely replaced LSH for embedding search.

**Inverted File Index (IVF)** clusters the dataset into $C$ cells using k-means (or similar), then at query time probes the $n_{\text{probe}}$ closest cluster centroids and does a brute-force scan within those cells only. The probe set is determined by $\text{argmin}_c \|\mathbf{q} - \boldsymbol{\mu}_c\|$ over the centroid set $\{\boldsymbol{\mu}_1, \ldots, \boldsymbol{\mu}_C\}$.

With $C = \sqrt{n}$ clusters and $n_{\text{probe}} = \sqrt{C}$, each query touches approximately $\sqrt{n}$ vectors instead of $n$, transforming $O(n)$ to $O(\sqrt{n})$ at the cost of some missed neighbors that live in unprobed cells. This is $10^3 \times$ faster than brute force at $n = 10^6$ with reasonable recall.

**Graph-based methods** are where the state of the art currently is. The intuition is that if you build a graph where each node is connected to its approximate nearest neighbors, you can navigate from any starting point toward any query by repeatedly moving to the closest neighbor among the current node's connections. This is the small-world network idea applied to metric spaces.

---

### HNSW: the algorithm behind Piramid's index

[Hierarchical Navigable Small World (HNSW)](https://arxiv.org/abs/1603.09320), proposed by Malkov and Yashunin in 2018, is currently the dominant algorithm for in-memory ANN search. It's what Piramid uses as its primary index type and deserves a proper explanation.

The core insight behind NSW (the non-hierarchical predecessor) is that if you build a graph by inserting nodes sequentially and connecting each new node to its nearest neighbors at the time of insertion, you get a *navigable small world* graph. Navigable means that greedy routing (always move to whichever neighbor is closest to the query) converges to the true nearest neighbor in $O(\log n)$ steps instead of $O(n)$.

The problem with flat NSW is that early-inserted nodes become overly central in the graph (they have many connections added later during other nodes' insertions), creating a hub structure that slows down routing. HNSW solves this by adding a hierarchy of layers.

**Layer construction.** Each node gets a maximum layer $l_{\max}$ drawn from an exponential distribution:

$$l_{\max} \sim \lfloor -\ln(\text{Uniform}(0,1)) \cdot m_l \rfloor, \quad m_l = \frac{1}{\ln M}$$

where $M$ is the target number of connections per node. This gives layer 0 all nodes, layer 1 roughly $1/M$ of them, layer 2 roughly $1/M^2$, and so on. At layer $l$, each node maintains at most $M$ bidirectional connections (or $M_{\max} = 2M$ at layer 0, since layer 0 carries the full resolution of the graph).

For $M = 16$ (Piramid's default), $m_l = 1/\ln(16) \approx 0.36$. The probability of a node reaching layer $l$ is $e^{-l/m_l} = M^{-l}$: so about 6% of nodes appear at layer 1, 0.4% at layer 2, and so on. The top layers are sparse and can be traversed in very few hops.

**Search algorithm.** Given a query $\mathbf{q}$ and target $k$, the search works in two phases:

1. **Greedy descent**: start from the entry point (the node at the highest layer). At each layer $l$ from $l_{\max}$ down to layer 1, do a greedy local search with $ef = 1$, finding the single closest node in the current layer and moving to it. This descent narrows the entry point of the fine search from the global graph down to a local region that's close to $\mathbf{q}$ in the original space.

2. **Layer-0 beam search**: at layer 0, run a beam search with parameter $ef \geq k$. Maintain a candidate set (min-heap by distance) and a nearest set (max-heap of size $ef$). Pop the closest candidate, explore its layer-0 neighbors, add any unvisited neighbor that's closer than the current furthest neighbor in the nearest set. Stop when the closest unvisited candidate is further than the worst nearest neighbor. Return the top $k$ from the nearest set.

The complexity of this search is $O(\log n)$ for the greedy descent and roughly $O(ef \cdot M \cdot d)$ for the beam search, dominated by the $ef \times M$ distance computations at layer 0.

The recall/speed tradeoff is entirely controlled by $ef$: larger $ef$ explores more candidates, finds better neighbors, but costs more computation. The construction-time $ef_{\text{construction}}$ controls graph quality during build; the query-time $ef_{\text{search}}$ controls query quality at search time. You can have a well-built graph (high $ef_{\text{construction}}$) and then lower $ef_{\text{search}}$ for fast lower-quality queries, or raise it for high-recall searches on the same graph.

> **The graph property that makes this work:** HNSW graphs approximate [*Delaunay graphs*](https://en.wikipedia.org/wiki/Delaunay_triangulation) at each layer, a structure where edges connect nodes that are "natural neighbors" of each other (no other node sits geometrically between them). Delaunay graphs have the property that greedy routing always converges. The select-neighbors heuristic during construction tries to maintain this: when pruning a node's connections to keep only $M$, it prefers neighbors that are "diverse" in direction rather than the $M$ nearest by distance alone. This keeps the graph navigable even in high-dimensional spaces where the nearest neighbor cluster is very tight.

![HNSW greedy search — the query descends from the sparse top layers to the dense layer 0, narrowing the candidate region at each step before running the full beam search](https://www.pinecone.io/_next/image/?url=https%3A%2F%2Fcdn.sanity.io%2Fimages%2Fvr8gru94%2Fproduction%2Fdc5cb11ea197ceb4e1f18214066c8c51526b9af5-1920x1080.png&w=3840&q=75)

---

### Piramid's search engine

The actual query pipeline in Piramid is implemented in `src/search/engine.rs`. It's a clean three-step flow: overfetch from the ANN index, score, filter.

**Step 1: Overfetch.** If a metadata filter is attached to the query, Piramid can't know in advance how many of the ANN candidates will survive the filter. If it only fetched $k$ candidates and the filter rejects 80% of them, you'd get far fewer than $k$ results. The fix is to fetch more:

```rust
let search_k = if params.filter.is_some() { 
    k.saturating_mul(expansion) 
} else { 
    k 
};
```

The `expansion` factor is the `filter_overfetch` from the search config (default 5), or a per-request override. So if you ask for top-10 with a filter, Piramid fetches 50 candidates from the ANN index, scores and filters all 50, then returns the best 10 that survived.

**Step 2: Score.** For each candidate ID returned by the ANN index, the engine retrieves the full float32 vector and calculates the exact similarity score using the configured metric. This is done on the raw vectors in memory with no approximation at scoring time, even if the index search itself was approximate. The scoring is exact even when the retrieval is ANN.

**Step 3: Filter and truncate.** If a filter is present, it's applied to the scored hits before sorting and truncating. The filter is a chain of conditions with AND semantics:

```rust
Filter::new()
    .eq("language", "en")
    .gte("year", 2020)
    .is_in("category", vec!["science", "tech"])
```

All conditions must pass. Range, equality, not-equal, and set membership (`in`) are all supported. The filter is applied post-ANN, which means it operates on metadata stored in memory, a fast in-process lookup with no secondary I/O.

**Batch search.** For multiple simultaneous queries, Piramid has a `search_batch_collection` function that reuses the same vector and metadata maps across all queries in the batch. When `parallel_search` is enabled (backed by [Rayon](https://github.com/rayon-rs/rayon)), queries within a batch execute in parallel across available CPU threads:

```rust
if storage.config().parallelism.parallel_search {
    queries
        .par_iter()
        .map(|query| search_collection_with_maps(...))
        .collect()
}
```

Each query is independent — they share only immutable references to the vector store, so the parallelism is lock-free.

### SearchConfig: the recall/speed dial

The `SearchConfig` struct controls the recall/speed tradeoff at query time. There are three named presets:

| Preset | `ef` (HNSW) | `nprobe` (IVF) | Use case |
|--------|-------------|----------------|----------|
| `fast` | 50 | 1 | Low-latency, recall can be approximate |
| `balanced` | default | default | General use |
| `high` | 400 | 20 | High-recall, latency less critical |

You can also pass a `SearchConfig` override per request via `SearchParams` without changing the collection's default config. This matters for cases like: the collection normally runs at `balanced`, but a specific important query needs `high` recall, so just pass a `search_config_override` in the params and that one query runs at `ef=400` without touching global settings.

The `ef` parameter is the most important knob. A useful empirical rule: at $ef = M \cdot k$ (so for $M = 16$, $k = 10$: $ef = 160$) you typically see Recall@10 above 95% on well-distributed embedding spaces. Pushing to $ef = 400$ usually reaches 99%+. The cost is roughly linear in $ef$: doubling $ef$ roughly doubles query time.

> **One subtlety:** when `search_config_override` changes `ef`, the overfetch stack still applies on top. If you're using a filter with `expansion = 5` and kick off a query for $k = 10$ with `ef = 50`, Piramid actually runs the HNSW search with `ef = max(50, 5 * 10) = 50` and fetches 50 candidates to hand to the filter. If you raise to $ef = 400$, you get 50 filter candidates from 400 explored nodes, giving much better graph coverage for the filtered result set.

---

> Piramid also supports a flat brute-force index (no ANN approximation), which is useful for small collections where recall must be exact, or for testing and benchmarking. Flat search always touches every vector and computes the exact distance, so recall is 1.0 by construction. The tradeoff is linear query time, which is fine for $n < 10^4$ or so but impractical beyond that. The SearchConfig parameters (`ef`, `nprobe`) are ignored for the flat index since there's nothing to tune when it always scans everything.