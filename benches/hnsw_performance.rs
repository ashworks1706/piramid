use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use piramid::storage::{VectorStorage, VectorEntry};
use piramid::metrics::Metric;

// helper to create random vector 
fn random_vector(dim: usize) -> Vec<f32> {
    (0..dim).map(|_| rand::random::<f32>()).collect()
}

// benchmarking insert vectors 
fn bench_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert");

    for size in [1_000, 10_000, 100_000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let _ = std::fs::remove_file("bench_insert.db");
                let _ = std::fs::remove_file(".hnsw.db");
                let mut storage = VectorStorage::open("bench_insert.db").unwrap();
                for _ in 0..size {
                    let vec = random_vector(128);
                    let entry = VectorEntry::new(vec, "doc".to_string());
                    storage.store(entry).unwrap();
                }
            });
        });
    }

    group.finish();
    let _ = std::fs::remove_file("bench_insert.db");
    let _ = std::fs::remove_file(".hnsw.db");
}


fn bench_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("search");

    for size in [1_000, 10_000, 100_000].iter() {
        // setup storage with size vectors
        let _ = std::fs::remove_file("bench_search.db");
        let _ = std::fs::remove_file(".hnsw.db");
        let mut storage = VectorStorage::open("bench_search.db").unwrap();
        println!("Preparing Dataset! Size: {}", size);
        for _ in 0..*size {
            let vec = random_vector(128);
            let entry = VectorEntry::new(vec, "doc".to_string());
            storage.store(entry).unwrap();
        }

        println!("Dataset Prepared! Size: {}", size);

        let query = random_vector(128);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &_size| {
            b.iter(|| {
                storage.search(&query, 10, Metric::Cosine)
            });
        });

        let _ = std::fs::remove_file("bench_search.db");
        let _ = std::fs::remove_file(".hnsw.db");
    }

    group.finish();
}
criterion_group!(benches, bench_insert, bench_search); // define benchmark group
criterion_main!(benches); // main function for benchmarks
