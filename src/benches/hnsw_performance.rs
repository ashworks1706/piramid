use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use piramid::storage::{VectorStorage, VectorEntry};
use piramid::metrics::Metric;
use uuid::Uuid;

// helper to create random vector 
fn random_vector(dim : usize) -> Vec<f32> {
    (0..dim).map(|_| rand::random::<f32>()).collect() // generate random float vector, 0..dim means
                                                      // length of vector
}

// benchmarking insert vectors 
fn bench_insert( c: &mut Criterion) {
    let mut group = c.benchmark_group("insert");

    for size in [1_000, 10_000, 100_000].iter() { // 1_000 means 1000 vectors, why _ ? because in rust you can use _ to separate thousands for better readability
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let _ = std::fs::remove_file("bench_insert.db"); // remove existing db file if any
                let _ = std::fs::remove_file(".hnsw.db");
                let mut storage = VectorStorage::new("bench_insert.db").unwrap();
                for _ in 0..size { // 0..size means from 0 to size-1
                    let vec = random_vector(128); // 128 dimension vector
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
                let _results = storage.search(&query, 10, Metric::Cosine).unwrap();
            });
        });

        let _ = std::fs::remove_file("bench_search.db");
        let _ = std::fs::remove_file(".hnsw.db");
    }

    group.finish();
}
criterion_group!(benches, bench_insert, bench_search); // define benchmark group
criterion_main!(benches); // main function for benchmarks
