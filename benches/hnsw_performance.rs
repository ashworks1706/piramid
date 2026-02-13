use criterion::{criterion_group, criterion_main, Criterion};
use piramid::{HnswConfig, HnswIndex};
use uuid::Uuid;
use std::collections::HashMap;

fn hnsw_insert_search_bench(c: &mut Criterion) {
    let config = HnswConfig::default();
    let mut index = HnswIndex::new(config);
    let mut vectors = HashMap::new();

    // Seed a small set of vectors
    for i in 0..1_000 {
        let id = Uuid::new_v4();
        let vec = vec![i as f32, (i * 2) as f32, (i * 3) as f32];
        vectors.insert(id, vec.clone());
        index.insert(id, &vec, &vectors);
    }

    let query = vec![10.0, 20.0, 30.0];

    c.bench_function("hnsw_search_1k", |b| {
        b.iter(|| {
            let _ = index.search(&query, 10, 200, &vectors);
        })
    });
}

criterion_group!(benches, hnsw_insert_search_bench);
criterion_main!(benches);
