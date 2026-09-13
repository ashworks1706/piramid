#![allow(clippy::unwrap_used, clippy::expect_used, reason = "benchmark setup")]
#![allow(
    missing_docs,
    reason = "criterion_group generates the undocumented harness functions"
)]
//! The exact scan on its two paths: slab hands the buffer straight in, gathered copies rows first.

use std::collections::HashMap;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use uuid::Uuid;

use piramid_core::metadata::Metadata;
use piramid_core::Document;
use piramid_database::search::{search, SearchParams, SearchTarget};
use piramid_database::storage::{HashMapVectorReader, VectorReader};
use piramid_database::ResidentManager;
use piramid_hardware::compute::Metric;

fn rows(count: usize, dim: usize) -> Vec<(Uuid, Vec<f32>)> {
    (0..count)
        .map(|i| {
            let f = i as f32;
            (
                Uuid::new_v4(),
                (0..dim).map(|d| ((f + d as f32) % 17.0) - 8.0).collect(),
            )
        })
        .collect()
}

fn exact_scan(c: &mut Criterion) {
    let empty_meta: HashMap<Uuid, Metadata> = HashMap::new();
    let document = Document::new(vec![0.0], String::new());
    let resolve = |_: &Uuid| Ok(Some(document.clone()));
    let mut group = c.benchmark_group("exact_scan");

    for (count, dim) in [(1_000, 384), (8_192, 384), (8_192, 1_536)] {
        let data = rows(count, dim);
        let query: Vec<f32> = (0..dim).map(|d| (d as f32 % 11.0) - 5.0).collect();

        let mut resident = ResidentManager::new();
        for (id, vector) in &data {
            resident.put_vector(*id, vector).unwrap();
        }
        let map: HashMap<Uuid, Vec<f32>> = data.iter().cloned().collect();
        let scattered = HashMapVectorReader::new(&map);
        assert!(
            resident.as_slab().is_some(),
            "the contiguous arm needs a slab"
        );
        assert!(scattered.as_slab().is_none(), "the gather arm must gather");

        let label = format!("{count}x{dim}");
        group.throughput(Throughput::Elements(count as u64));

        for (name, vectors) in [
            ("slab", &resident as &dyn VectorReader),
            ("gathered", &scattered),
        ] {
            let target = SearchTarget {
                vectors,
                metadata: &empty_meta,
            };
            group.bench_with_input(BenchmarkId::new(name, &label), &(), |b, ()| {
                b.iter(|| {
                    search(
                        &target,
                        &query,
                        10,
                        Metric::Cosine,
                        SearchParams::default(),
                        &resolve,
                    )
                    .unwrap()
                });
            });
        }
    }
    group.finish();
}

criterion_group!(benches, exact_scan);
criterion_main!(benches);
