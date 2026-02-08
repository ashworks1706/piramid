# Piramid Benchmarks

Official performance benchmarks using [Criterion.rs](https://github.com/bheisler/criterion.rs).

## Running Benchmarks

```bash
# Run all benchmarks (takes ~10-15 minutes)
cargo bench

# Run specific benchmark
cargo bench --bench hnsw_performance
cargo bench --bench hnsw_accuracy

# View HTML reports
open target/criterion/report/index.html
```

 

## Benchmark Suites

### 1. **Performance Benchmarks** (`hnsw_performance.rs`)

Measures speed and throughput.

**Tests:**
- **Insert**: Vectors per second at 1k, 5k, 10k dataset sizes
- **Search**: Latency (ms) per query at 1k, 5k, 10k dataset sizes

**Metrics:**
- Time per operation
- Throughput (ops/sec)
- Statistical analysis (mean, median, std dev)

 

### 2. **Accuracy Benchmarks** (`hnsw_accuracy.rs`)

Measures search quality.

**Tests:**
- **Recall@10**: Percentage of true top-10 results found by HNSW

**Dataset:**
- 5,000 vectors (128 dimensions)
- 50 test queries
- Compares HNSW results vs brute-force ground truth

**Expected Results:**
- Recall@10: >90% (good)
- Note: Random vectors have low recall (no structure)

 

## Configuration

**Vector dimensions:** 128  
**HNSW parameters:**
- M = 16 (connections per node)
- ef_construction = 200
- ef_search = 2*k (dynamic)

**Distance metric:** Cosine similarity

 

## Interpreting Results

### Performance:
- **Good**: Insert >500 vectors/sec, Search <10ms
- **Excellent**: Insert >1000 vectors/sec, Search <5ms

### Accuracy:
- **Good**: Recall@10 >90%
- **Excellent**: Recall@10 >95%

 

## Output Location

```
target/criterion/
├── report/
│   └── index.html          ← Open this for graphs
├── insert/
│   ├── 1000/
│   ├── 5000/
│   └── 10000/
└── search/
    ├── 1000/
    ├── 5000/
    └── 10000/
```

 

## Benchmark Details

### Sample Output:

```
insert/1000             time:   [1.57 s 1.60 s 1.63 s]
                        change: [-2.34% +0.12% +2.89%]

search/10000            time:   [3.45 ms 3.52 ms 3.59 ms]
                        change: [-5.21% -3.84% -2.47%]

recall@10               Average: 92.4%
```

### What This Means:

- **time: [min mean max]** - 95% confidence interval
- **change: [...]** - Performance vs previous run
- **Average:** Mean across test queries

 

See [Criterion docs](https://bheisler.github.io/criterion.rs/book/) for more.

 
