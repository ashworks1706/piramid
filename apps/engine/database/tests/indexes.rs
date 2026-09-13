#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]
//! The flat, HNSW and IVF indexes and index selection.

use piramid_database::index::{
    FlatConfig, FlatIndex, HashMapVectorReader, HnswConfig, HnswIndex, IndexConfig, IndexKind,
    IndexSearchRequest, IvfConfig, IvfIndex, VectorIndex,
};
use std::collections::HashMap;
use uuid::Uuid;

#[test]
fn flat_index_searches() {
    let mut idx = FlatIndex::new(FlatConfig::default());
    let mut vectors = HashMap::new();

    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();
    let v1 = vec![1.0, 0.0, 0.0];
    let v2 = vec![0.0, 1.0, 0.0];
    vectors.insert(id1, v1.clone());
    vectors.insert(id2, v2.clone());
    let reader = HashMapVectorReader::new(&vectors);

    idx.insert(id1, &v1, &reader).unwrap();
    let bootstrap_stats = idx.stats();
    assert_eq!(bootstrap_stats.total_vectors, 1);

    idx.insert(id2, &v2, &reader).unwrap();
    let ready_stats = idx.stats();
    assert_eq!(ready_stats.total_vectors, 2);

    let empty_meta: HashMap<Uuid, piramid_core::metadata::Metadata> = HashMap::new();
    let results = idx
        .search(IndexSearchRequest::new(
            &v1,
            1,
            &reader,
            piramid_core::config::SearchConfig::default(),
            &empty_meta,
        ))
        .unwrap();
    assert_eq!(results.first(), Some(&id1));
}

#[test]
fn hnsw_tombstone_tracks() {
    let mut idx = HnswIndex::new(HnswConfig::default());
    let mut vectors = HashMap::new();

    let id = Uuid::new_v4();
    let vec = vec![1.0, 2.0, 3.0];
    vectors.insert(id, vec.clone());
    let reader = HashMapVectorReader::new(&vectors);
    idx.insert(id, &vec, &reader).unwrap();

    let empty_meta: HashMap<Uuid, piramid_core::metadata::Metadata> = HashMap::new();
    let results = idx.search(&vec, 1, 50, &reader, None, &empty_meta).unwrap();
    assert!(!results.is_empty());

    idx.remove(&id);
    let stats = idx.stats();
    assert_eq!(stats.tombstones, 1);
    assert_eq!(stats.total_nodes, 0);
}

#[test]
fn ivf_search_basic() {
    let config = IvfConfig {
        num_clusters: 2,
        ..IvfConfig::default()
    };
    let mut idx = IvfIndex::new(config);
    let mut vectors = HashMap::new();

    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();
    let v1 = vec![1.0, 0.0, 0.0];
    let v2 = vec![0.9, 0.1, 0.0];
    vectors.insert(id1, v1.clone());
    {
        let reader = HashMapVectorReader::new(&vectors);
        idx.insert(id1, &v1, &reader).unwrap();
    }
    let bootstrap_stats = idx.stats();
    assert_eq!(bootstrap_stats.total_vectors, 1);

    vectors.insert(id2, v2.clone());
    let reader = HashMapVectorReader::new(&vectors);
    idx.insert(id2, &v2, &reader).unwrap();
    let ready_stats = idx.stats();
    assert_eq!(ready_stats.total_vectors, 2);

    let empty_meta: HashMap<Uuid, piramid_core::metadata::Metadata> = HashMap::new();
    let results = idx
        .search(IndexSearchRequest::new(
            &v1,
            1,
            &reader,
            piramid_core::config::SearchConfig::default(),
            &empty_meta,
        ))
        .unwrap();
    assert!(!results.is_empty());
}

#[test]
fn ivf_search_fails_before_clusters_are_ready() {
    let config = IvfConfig {
        num_clusters: 4,
        ..IvfConfig::default()
    };
    let mut idx = IvfIndex::new(config);
    let mut vectors = HashMap::new();

    let id = Uuid::new_v4();
    let vec = vec![1.0, 0.0, 0.0];
    vectors.insert(id, vec.clone());
    let reader = HashMapVectorReader::new(&vectors);
    idx.insert(id, &vec, &reader).unwrap();
    assert_eq!(idx.stats().total_vectors, 1);

    let empty_meta: HashMap<Uuid, piramid_core::metadata::Metadata> = HashMap::new();
    let result = idx.search(IndexSearchRequest::new(
        &vec,
        1,
        &reader,
        piramid_core::config::SearchConfig::default(),
        &empty_meta,
    ));
    assert!(result.is_err());

    idx.remove(&id);
    assert_eq!(idx.stats().total_vectors, 0);
}

#[test]
fn ivf_duplicate_insert_uses_id_map_without_duplicate_membership() {
    let config = IvfConfig {
        num_clusters: 2,
        ..IvfConfig::default()
    };
    let mut idx = IvfIndex::new(config);
    let mut vectors = HashMap::new();

    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();
    let v1 = vec![1.0, 0.0, 0.0];
    let v2 = vec![0.0, 1.0, 0.0];

    vectors.insert(id1, v1.clone());
    {
        let reader = HashMapVectorReader::new(&vectors);
        idx.insert(id1, &v1, &reader).unwrap();
        idx.insert(id1, &v1, &reader).unwrap();
    }
    assert_eq!(idx.stats().total_vectors, 1);

    vectors.insert(id2, v2.clone());
    let reader = HashMapVectorReader::new(&vectors);
    idx.insert(id2, &v2, &reader).unwrap();
    idx.insert(id2, &v2, &reader).unwrap();

    let stats = idx.stats();
    assert_eq!(stats.total_vectors, 2);
    match stats.details {
        piramid_database::index::IndexDetails::Ivf {
            vectors_per_cluster,
            ..
        } => {
            let indexed_memberships: usize = vectors_per_cluster.iter().sum();
            assert_eq!(indexed_memberships, 2);
        }
        other => panic!("expected IVF stats, got {other:?}"),
    }
}

#[test]
fn index_selector_prefers_expected_types() {
    let cfg = IndexConfig::default();
    assert_eq!(cfg.select_type(1_000), IndexKind::Flat);
    assert_eq!(cfg.select_type(50_000), IndexKind::Ivf);
    assert_eq!(cfg.select_type(500_000), IndexKind::Hnsw);
}

// Under auto, an ivf_num_clusters without ivf_num_probes sizes the probes from that cluster count.
#[test]
fn auto_ivf_probes_are_sized_from_the_configured_cluster_count() {
    use piramid_database::index::{create_index, AutoIndexConfig, IndexDetails};

    let cfg = IndexConfig::Auto {
        metric: piramid_hardware::compute::Metric::Cosine,
        auto: AutoIndexConfig {
            flat_max_vectors: 1,
            ivf_max_vectors: 1_000_000,
            ivf_num_clusters: Some(4),
            ivf_num_probes: None,
            ..AutoIndexConfig::default()
        },
    };
    let index = create_index(&cfg, piramid_core::config::ExecutionMode::default(), 10_000);
    match index.stats().details {
        IndexDetails::Ivf { num_probes, .. } => assert_eq!(num_probes, 1),
        other => panic!("expected IVF stats, got {other:?}"),
    }
}

// HNSW evaluates filters during traversal.
#[test]
fn hnsw_search_applies_a_filter_during_traversal() {
    use piramid_core::metadata::{metadata, Filter, Metadata};

    let mut idx = HnswIndex::new(HnswConfig::default());
    let mut vectors = HashMap::new();
    let mut metadatas: HashMap<Uuid, Metadata> = HashMap::new();

    let mut wanted = Vec::new();
    for i in 0..40 {
        let id = Uuid::new_v4();
        let vector = vec![i as f32 * 0.01, 1.0, 0.0];
        let lang = if i % 2 == 0 { "rust" } else { "go" };
        if lang == "rust" {
            wanted.push(id);
        }
        vectors.insert(id, vector);
        metadatas.insert(id, metadata([("lang", lang.into())]));
    }

    let reader = HashMapVectorReader::new(&vectors);
    for (id, vector) in &vectors {
        idx.insert(*id, vector, &reader).unwrap();
    }

    let filter = Filter::new().eq("lang", "rust");
    let hits = idx
        .search(
            &[0.0, 1.0, 0.0],
            10,
            200,
            &reader,
            Some(&filter),
            &metadatas,
        )
        .unwrap();

    assert!(!hits.is_empty(), "filter must not exclude everything");
    for id in &hits {
        assert!(wanted.contains(id), "a 'go' document survived the filter");
    }

    // No filter reaches strictly more of the graph than a filtered search.
    let unfiltered = idx
        .search(&[0.0, 1.0, 0.0], 10, 200, &reader, None, &metadatas)
        .unwrap();
    assert!(unfiltered.len() >= hits.len());
}

/// The flat scan reaches the kernel by slab or by gather, and a metric applies batched or one
/// pair at a time. Every one of those paths ranks a collection the same way.
#[test]
fn every_flat_scoring_path_ranks_a_collection_the_same_way() {
    use piramid_core::config::CacheConfig;
    use piramid_database::index::VectorReader;
    use piramid_database::{CacheManager, VectorStore};
    use piramid_hardware::compute::strategies::for_mode;
    use piramid_hardware::compute::Metric;

    // Enough rows to cross the chunk boundary the gather path blocks on.
    let dim = 16;
    let rows: Vec<(Uuid, Vec<f32>)> = (0..2500)
        .map(|i| {
            let f = i as f32;
            (
                Uuid::new_v4(),
                (0..dim).map(|d| ((f + d as f32) % 7.0) - 3.0).collect(),
            )
        })
        .collect();
    let query: Vec<f32> = (0..dim).map(|d| (d as f32 % 5.0) - 2.0).collect();
    let empty_meta: HashMap<Uuid, piramid_core::metadata::Metadata> = HashMap::new();

    for metric in [Metric::Cosine, Metric::Euclidean, Metric::DotProduct] {
        let config = FlatConfig {
            metric,
            ..FlatConfig::default()
        };

        // Contiguous: the store buffer goes straight to the kernel.
        let mut cache = CacheManager::new(CacheConfig::default());
        let mut contiguous = FlatIndex::new(config);
        for (id, vector) in &rows {
            cache.put_vector(*id, vector).unwrap();
        }
        for (id, vector) in &rows {
            contiguous.insert(*id, vector, &cache).unwrap();
        }
        assert!(
            cache.as_slab().is_some(),
            "the store should be offering its buffer"
        );

        // Scattered: every block is gathered before it is scored.
        let map: HashMap<Uuid, Vec<f32>> = rows.iter().cloned().collect();
        let scattered_reader = HashMapVectorReader::new(&map);
        assert!(scattered_reader.as_slab().is_none());
        let mut scattered = FlatIndex::new(config);
        for (id, vector) in &rows {
            scattered.insert(*id, vector, &scattered_reader).unwrap();
        }

        // A hole withdraws the buffer, and the same index gathers instead.
        let mut holed = VectorStore::new();
        for (id, vector) in &rows {
            holed.put(*id, vector).unwrap();
        }
        let evicted = Uuid::new_v4();
        holed.put(evicted, &vec![0.0; dim]).unwrap();
        holed.remove(&evicted);
        assert!(holed.as_slab().is_none());

        let k = 10;
        let from_slab = contiguous
            .search(IndexSearchRequest::new(
                &query,
                k,
                &cache,
                piramid_core::config::SearchConfig::default(),
                &empty_meta,
            ))
            .unwrap();
        let from_gather = scattered
            .search(IndexSearchRequest::new(
                &query,
                k,
                &scattered_reader,
                piramid_core::config::SearchConfig::default(),
                &empty_meta,
            ))
            .unwrap();
        let from_holed = contiguous
            .search(IndexSearchRequest::new(
                &query,
                k,
                &holed,
                piramid_core::config::SearchConfig::default(),
                &empty_meta,
            ))
            .unwrap();

        assert_eq!(from_slab.len(), k);
        assert_eq!(from_slab, from_gather, "{metric:?}: slab vs gather");
        assert_eq!(from_slab, from_holed, "{metric:?}: slab vs holed fallback");

        // Compared against scoring one pair at a time.
        let kernels = for_mode(config.mode).unwrap();
        let mut pairwise: Vec<(Uuid, f32)> = rows
            .iter()
            .map(|(id, vector)| (*id, metric.calculate(&query, vector, kernels).unwrap()))
            .collect();
        pairwise.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let expected: Vec<Uuid> = pairwise.into_iter().take(k).map(|(id, _)| id).collect();
        assert_eq!(from_slab, expected, "{metric:?}: batch vs pairwise");
    }
}

/// An IVF search probing every partition scores each posting list in blocks, and ranks a
/// collection exactly as scoring one pair at a time would.
#[test]
fn ivf_probing_every_partition_ranks_as_pairwise_scoring_does() {
    use piramid_core::config::{CacheConfig, SearchConfig};
    use piramid_database::CacheManager;
    use piramid_hardware::compute::strategies::for_mode;
    use piramid_hardware::compute::Metric;

    // Two partitions over 2500 rows puts at least one posting list across a block boundary.
    let dim = 16;
    let rows: Vec<(Uuid, Vec<f32>)> = (0..2500)
        .map(|i| {
            let f = i as f32;
            (
                Uuid::new_v4(),
                (0..dim).map(|d| ((f + d as f32) % 7.0) - 3.0).collect(),
            )
        })
        .collect();
    let query: Vec<f32> = (0..dim).map(|d| (d as f32 % 5.0) - 2.0).collect();
    let empty_meta: HashMap<Uuid, piramid_core::metadata::Metadata> = HashMap::new();
    let map: HashMap<Uuid, Vec<f32>> = rows.iter().cloned().collect();
    let scattered = HashMapVectorReader::new(&map);
    let mut cache = CacheManager::new(CacheConfig::default());
    for (id, vector) in &rows {
        cache.put_vector(*id, vector).unwrap();
    }

    for metric in [Metric::Cosine, Metric::Euclidean, Metric::DotProduct] {
        let config = IvfConfig {
            num_clusters: 2,
            metric,
            ..IvfConfig::default()
        };
        let mut index = IvfIndex::new(config);
        index.build_clusters(&scattered).unwrap();

        let probe_all = SearchConfig {
            nprobe: Some(config.num_clusters),
            ..SearchConfig::default()
        };
        let k = 10;
        let from_scattered = index
            .search(IndexSearchRequest::new(
                &query,
                k,
                &scattered,
                probe_all,
                &empty_meta,
            ))
            .unwrap();
        let from_cache = index
            .search(IndexSearchRequest::new(
                &query,
                k,
                &cache,
                probe_all,
                &empty_meta,
            ))
            .unwrap();

        let kernels = for_mode(config.mode).unwrap();
        let mut pairwise: Vec<(Uuid, f32)> = rows
            .iter()
            .map(|(id, vector)| (*id, metric.calculate(&query, vector, kernels).unwrap()))
            .collect();
        pairwise.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let expected: Vec<(Uuid, f32)> = pairwise.into_iter().take(k).collect();

        assert_eq!(from_scattered.len(), k);
        assert_eq!(from_scattered, from_cache, "{metric:?}: scattered vs cache");
        // Rows repeat with period 7, so ties are compared by score and not by id.
        let score_of = |id: &Uuid| metric.calculate(&query, &map[id], kernels).unwrap();
        for (got, (_, want)) in from_scattered.iter().zip(&expected) {
            assert!(
                (score_of(got) - want).abs() < 1e-5,
                "{metric:?}: batch ranking diverges from pairwise"
            );
        }
    }
}

/// A posting list naming a vector the reader does not hold fails the search.
#[test]
fn ivf_search_fails_when_a_posting_list_names_a_missing_vector() {
    let config = IvfConfig {
        num_clusters: 2,
        ..IvfConfig::default()
    };
    let mut index = IvfIndex::new(config);
    let mut vectors = HashMap::new();
    let kept = Uuid::new_v4();
    let dropped = Uuid::new_v4();
    vectors.insert(kept, vec![1.0, 0.0, 0.0]);
    vectors.insert(dropped, vec![0.0, 1.0, 0.0]);
    index
        .build_clusters(&HashMapVectorReader::new(&vectors))
        .unwrap();

    vectors.remove(&dropped);
    let reader = HashMapVectorReader::new(&vectors);
    let empty_meta: HashMap<Uuid, piramid_core::metadata::Metadata> = HashMap::new();
    let result = index.search(IndexSearchRequest::new(
        &[1.0, 0.0, 0.0],
        2,
        &reader,
        piramid_core::config::SearchConfig {
            nprobe: Some(2),
            ..piramid_core::config::SearchConfig::default()
        },
        &empty_meta,
    ));
    assert!(result.is_err());
}

/// A stored zero vector under cosine scores NaN; every index leaves it out and still fills k.
#[test]
fn a_stored_zero_vector_is_left_out_of_cosine_results() {
    use piramid_core::config::SearchConfig;

    let mut vectors = HashMap::new();
    let zero_id = Uuid::new_v4();
    vectors.insert(zero_id, vec![0.0, 0.0, 0.0]);
    for i in 0..30 {
        let f = i as f32;
        vectors.insert(
            Uuid::new_v4(),
            vec![1.0 + f, (f % 5.0) - 2.0, (f % 3.0) - 1.0],
        );
    }
    let reader = HashMapVectorReader::new(&vectors);
    let empty_meta: HashMap<Uuid, piramid_core::metadata::Metadata> = HashMap::new();
    let query = [1.0, 0.5, 0.25];
    let k = 5;

    let mut flat = FlatIndex::new(FlatConfig::default());
    let mut hnsw = HnswIndex::new(HnswConfig::default());
    for (id, vector) in &vectors {
        flat.insert(*id, vector, &reader).unwrap();
        hnsw.insert(*id, vector, &reader).unwrap();
    }
    let mut ivf = IvfIndex::new(IvfConfig {
        num_clusters: 3,
        ..IvfConfig::default()
    });
    ivf.build_clusters(&reader).unwrap();
    let probe_all = SearchConfig {
        nprobe: Some(3),
        ..SearchConfig::default()
    };

    let from_flat = flat
        .search(IndexSearchRequest::new(
            &query,
            k,
            &reader,
            SearchConfig::default(),
            &empty_meta,
        ))
        .unwrap();
    let from_ivf = ivf
        .search(IndexSearchRequest::new(
            &query,
            k,
            &reader,
            probe_all,
            &empty_meta,
        ))
        .unwrap();
    let from_hnsw = hnsw
        .search(&query, k, 200, &reader, None, &empty_meta)
        .unwrap();
    for (name, ids) in [("flat", from_flat), ("ivf", from_ivf), ("hnsw", from_hnsw)] {
        assert_eq!(ids.len(), k, "{name}");
        assert!(!ids.contains(&zero_id), "{name} ranked the zero vector");
    }
}

#[test]
fn an_ivf_search_with_zero_probes_is_refused() {
    let config = IvfConfig {
        num_clusters: 1,
        ..IvfConfig::default()
    };
    let mut idx = IvfIndex::new(config);
    let mut vectors = HashMap::new();
    let id = Uuid::new_v4();
    vectors.insert(id, vec![1.0, 0.0]);
    let reader = HashMapVectorReader::new(&vectors);
    idx.insert(id, &[1.0, 0.0], &reader).unwrap();

    let empty_meta: HashMap<Uuid, piramid_core::metadata::Metadata> = HashMap::new();
    let search = piramid_core::config::SearchConfig {
        nprobe: Some(0),
        ..piramid_core::config::SearchConfig::default()
    };
    let error = idx
        .search(IndexSearchRequest::new(
            &[1.0, 0.0],
            1,
            &reader,
            search,
            &empty_meta,
        ))
        .unwrap_err();
    assert!(error.to_string().contains("nprobe must be >= 1"), "{error}");
}

#[test]
fn hnsw_mean_connections_count_layer_zero_and_are_absent_when_empty() {
    use piramid_database::index::IndexDetails;

    let config = HnswConfig {
        m: 2,
        m_max: 2,
        ml: 1.0 / 2.0_f32.ln(),
        ..HnswConfig::default()
    };
    let mut idx = HnswIndex::new(config);
    assert_eq!(idx.stats().avg_connections, None);

    let mut vectors = HashMap::new();
    let ids: Vec<Uuid> = (0..64).map(|_| Uuid::new_v4()).collect();
    for (i, id) in ids.iter().enumerate() {
        let angle = i as f32 * 0.1;
        vectors.insert(*id, vec![angle.cos(), angle.sin(), 1.0]);
    }
    let reader = HashMapVectorReader::new(&vectors);
    for id in &ids {
        idx.insert(*id, &vectors[id], &reader).unwrap();
    }

    let stats = idx.stats();
    let mean = stats.avg_connections.unwrap();
    assert!(stats.max_layer > 0, "upper layers exist");
    assert!(mean > 0.0 && mean <= config.m_max as f32, "{mean}");
    match VectorIndex::stats(&idx).details {
        IndexDetails::Hnsw {
            avg_connections, ..
        } => assert_eq!(avg_connections, Some(mean)),
        other => panic!("expected HNSW stats, got {other:?}"),
    }
}
