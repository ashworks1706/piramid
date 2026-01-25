use criterion::{criterion_group, criterion_main, Criterion};
use piramid::storage::{VectorStorage, VectorEntry};
use piramid::metrics::Metric;
use std::collections::HashSet;
use uuid::Uuid;

fn random_vector(dim: usize) -> Vec<f32> {
    (0..dim).map(|_| rand::random::<f32>()).collect()
}

// Calculate recall@k: how many of top-k results match ground truth
fn calculate_recall(hnsw_results: &[Uuid], truth_results: &[Uuid], k: usize) -> f32 {
    let hnsw_set: HashSet<_> = hnsw_results.iter().take(k).collect();
    let truth_set: HashSet<_> = truth_results.iter().take(k).collect();
    let intersection = hnsw_set.intersection(&truth_set).count();
    intersection as f32 / k as f32
}

fn bench_recall(c: &mut Criterion) {
    let _ = std::fs::remove_file("bench_recall.db");
    let _ = std::fs::remove_file(".hnsw.db");
    let mut storage = VectorStorage::open("bench_recall.db").unwrap();

    println!("Preparing dataset for recall test...");

    let dim = 128;
    let dataset_size = 10_000;

    for i in 0..dataset_size {
        let vec = random_vector(dim);
        let entry = VectorEntry::new(vec, format!("doc{}", i));
        storage.store(entry).unwrap();
    }

    // Test recall on multiple queries
    println!("Testing recall@10...");
    let mut total_recall = 0.0;
    let num_queries = 100;

    for _ in 0..num_queries {
        let query = random_vector(dim);

        // HNSW results (approximate)
        let hnsw_results = storage.search(&query, 10, Metric::Cosine);
        let hnsw_ids: Vec<_> = hnsw_results.iter().map(|r| r.id).collect();

        // Brute force ground truth (exact)
        let mut all_results = storage.get_all()
            .into_iter()
            .map(|entry| {
                let score = Metric::Cosine.calculate(&query, &entry.vector);
                (entry.id, score)
            })
            .collect::<Vec<_>>();

        all_results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let truth_ids: Vec<_> = all_results.iter().map(|(id, _)| *id).collect();

        let recall = calculate_recall(&hnsw_ids, &truth_ids, 10);

        total_recall += recall;
    }

    let avg_recall = total_recall / num_queries as f32;

    println!("Average Recall@10: {:.2}%", avg_recall * 100.0);

    c.bench_function("recall@10", |b| {
        b.iter(|| {
            avg_recall
        });
    });

    let _ = std::fs::remove_file("bench_recall.db");
    let _ = std::fs::remove_file(".hnsw.db");
}

criterion_group!(benches, bench_recall);
criterion_main!(benches);
