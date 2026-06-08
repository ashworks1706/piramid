use piramid::{
    index::{
        FlatConfig, FlatIndex, HnswConfig, HnswIndex, IndexConfig, IndexType, IvfConfig, IvfIndex,
    },
    HashMapVectorReader, VectorIndex,
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

    idx.insert(id1, &v1, &reader);
    idx.insert(id2, &v2, &reader);

    let empty_meta: HashMap<Uuid, piramid::metadata::Metadata> = HashMap::new();
    let results = idx
        .search(
            &v1,
            1,
            &reader,
            piramid::config::SearchConfig::default(),
            None,
            &empty_meta,
        )
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
    idx.insert(id, &vec, &reader);

    let empty_meta: HashMap<Uuid, piramid::metadata::Metadata> = HashMap::new();
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
    vectors.insert(id2, v2.clone());
    let reader = HashMapVectorReader::new(&vectors);

    idx.insert(id1, &v1, &reader);
    idx.insert(id2, &v2, &reader);

    let empty_meta: HashMap<Uuid, piramid::metadata::Metadata> = HashMap::new();
    let results = idx
        .search(
            &v1,
            1,
            &reader,
            piramid::config::SearchConfig::default(),
            None,
            &empty_meta,
        )
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
    idx.insert(id, &vec, &reader);

    let empty_meta: HashMap<Uuid, piramid::metadata::Metadata> = HashMap::new();
    let result = idx.search(
        &vec,
        1,
        &reader,
        piramid::config::SearchConfig::default(),
        None,
        &empty_meta,
    );
    assert!(result.is_err());
}

#[test]
fn index_selector_prefers_expected_types() {
    let cfg = IndexConfig::default();
    assert_eq!(cfg.select_type(1_000), IndexType::Flat);
    assert_eq!(cfg.select_type(50_000), IndexType::Ivf);
    assert_eq!(cfg.select_type(500_000), IndexType::Hnsw);
}
