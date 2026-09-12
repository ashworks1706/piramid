#![allow(clippy::unwrap_used, clippy::expect_used, reason = "benchmark setup")]
#![allow(
    missing_docs,
    reason = "criterion_group generates the undocumented harness functions"
)]
//! The IVF posting-list scan, probing every partition so the scan dominates the query.

use std::collections::HashMap;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use uuid::Uuid;

use piramid_core::config::{CacheConfig, IvfConfig, SearchConfig};
use piramid_core::metadata::Metadata;
use piramid_database::index::{IndexSearchRequest, IvfIndex, VectorIndex};
use piramid_database::CacheManager;

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

fn ivf_scan(c: &mut Criterion) {
    let empty_meta: HashMap<Uuid, Metadata> = HashMap::new();
    let mut group = c.benchmark_group("ivf_scan");

    for (count, dim) in [(8_192, 384), (8_192, 1_536)] {
        let data = rows(count, dim);
        let query: Vec<f32> = (0..dim).map(|d| (d as f32 % 11.0) - 5.0).collect();

        let mut cache = CacheManager::new(CacheConfig::default());
        for (id, vector) in &data {
            cache.put_vector(*id, vector).unwrap();
        }
        let config = IvfConfig {
            num_clusters: 16,
            max_iterations: 2,
            ..IvfConfig::default()
        };
        let mut index = IvfIndex::new(config);
        index.build_clusters(&cache).unwrap();
        let probe_all = SearchConfig {
            nprobe: Some(config.num_clusters),
            ..SearchConfig::default()
        };

        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::new("probe_all", format!("{count}x{dim}")),
            &(),
            |b, ()| {
                b.iter(|| {
                    index
                        .search(IndexSearchRequest::new(
                            &query,
                            10,
                            &cache,
                            probe_all,
                            &empty_meta,
                        ))
                        .unwrap()
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, ivf_scan);
criterion_main!(benches);
