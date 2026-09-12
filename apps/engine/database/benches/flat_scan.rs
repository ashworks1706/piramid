#![allow(clippy::unwrap_used, clippy::expect_used, reason = "benchmark setup")]
#![allow(
    missing_docs,
    reason = "criterion_group generates the undocumented harness functions"
)]
//! The flat scan on its two paths to the kernel.
//!
//! slab hands the store's own buffer to one batch call. gathered copies each block of rows into
//! scratch first. Same kernel, same metric, same data, so the layout is what varies.

use std::collections::HashMap;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use uuid::Uuid;

use piramid_core::config::{CacheConfig, FlatConfig, SearchConfig};
use piramid_core::metadata::Metadata;
use piramid_database::index::{
    FlatIndex, HashMapVectorReader, IndexSearchRequest, VectorIndex, VectorReader,
};
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

fn flat_scan(c: &mut Criterion) {
    let empty_meta: HashMap<Uuid, Metadata> = HashMap::new();
    let mut group = c.benchmark_group("flat_scan");

    for (count, dim) in [(1_000, 384), (8_192, 384), (8_192, 1_536)] {
        let data = rows(count, dim);
        let query: Vec<f32> = (0..dim).map(|d| (d as f32 % 11.0) - 5.0).collect();

        let mut cache = CacheManager::new(CacheConfig::default());
        for (id, vector) in &data {
            cache.put_vector(*id, vector).unwrap();
        }
        let map: HashMap<Uuid, Vec<f32>> = data.iter().cloned().collect();
        let scattered = HashMapVectorReader::new(&map);

        let mut index = FlatIndex::new(FlatConfig::default());
        for (id, vector) in &data {
            index.insert(*id, vector, &cache).unwrap();
        }
        assert!(cache.as_slab().is_some(), "the contiguous arm needs a slab");
        assert!(scattered.as_slab().is_none(), "the gather arm must gather");

        let label = format!("{count}x{dim}");
        group.throughput(Throughput::Elements(count as u64));

        group.bench_with_input(BenchmarkId::new("slab", &label), &(), |b, ()| {
            b.iter(|| {
                index
                    .search(IndexSearchRequest::new(
                        &query,
                        10,
                        &cache,
                        SearchConfig::default(),
                        &empty_meta,
                    ))
                    .unwrap()
            });
        });

        group.bench_with_input(BenchmarkId::new("gathered", &label), &(), |b, ()| {
            b.iter(|| {
                index
                    .search(IndexSearchRequest::new(
                        &query,
                        10,
                        &scattered,
                        SearchConfig::default(),
                        &empty_meta,
                    ))
                    .unwrap()
            });
        });
    }
    group.finish();
}

criterion_group!(benches, flat_scan);
criterion_main!(benches);
