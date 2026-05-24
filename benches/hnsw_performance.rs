use criterion::{criterion_group, criterion_main, Criterion};
use piramid::{HashMapVectorReader, HnswConfig, HnswIndex, Metadata};
use std::collections::HashMap;
use uuid::Uuid;

fn hnsw_insert_search_bench(c: &mut Criterion) {
    let config = HnswConfig::default();
    let mut index = HnswIndex::new(config);
    let mut vectors = HashMap::new();
    let metadatas: HashMap<Uuid, Metadata> = HashMap::new();

    for i in 0..1_0000 {
        let id = Uuid::new_v4();
        let vec = vec![i as f32, (i * 2) as f32, (i * 3) as f32];
        vectors.insert(id, vec.clone());
        let reader = HashMapVectorReader::new(&vectors);
        index.insert(id, &vec, &reader);
    }

    let query = vec![10.0, 20.0, 30.0];

    c.bench_function("hnsw_search_10k", |b| {
        b.iter(|| {
            let reader = HashMapVectorReader::new(&vectors);
            let _ = index.search(&query, 10, 200, &reader, None, &metadatas);
        })
    });
}

criterion_group!(benches, hnsw_insert_search_bench);
criterion_main!(benches);
