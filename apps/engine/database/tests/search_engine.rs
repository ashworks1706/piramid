#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]
//! Search over a collection: filters, metrics and thresholds.

use std::fs;
use {
    piramid_core::metadata::metadata, piramid_core::metadata::Filter, piramid_core::Document,
    piramid_database::search::SearchParams, piramid_database::Collection,
    piramid_hardware::compute::Metric,
};

fn cleanup(path: &str) {
    let sidecars = [
        format!("{path}.offsets.db"),
        format!("{path}.wal.db"),
        format!("{path}.vecindex.db"),
        format!("{path}.manifest.db"),
    ];
    for p in std::iter::once(path.to_string()).chain(sidecars) {
        let _ = fs::remove_file(p);
    }
}

#[test]
fn search_respects_filter() {
    let test_db = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_search_filter.db");
    cleanup(test_db);

    {
        let mut storage = Collection::open(test_db).unwrap();

        let e1 = Document::with_metadata(
            vec![1.0, 0.0, 0.0],
            "rust doc".to_string(),
            metadata([("lang", "rust".into())]),
        );
        let e2 = Document::with_metadata(
            vec![0.9, 0.1, 0.0],
            "python doc".to_string(),
            metadata([("lang", "python".into())]),
        );

        storage.insert(e1).unwrap();
        storage.insert(e2).unwrap();

        let filter = Filter::new().eq("lang", "rust");
        let params = SearchParams {
            mode: storage.config().execution,
            filter: Some(&filter),
            search_config_override: None,
            min_score: None,
        };

        let results = storage
            .search(&[1.0, 0.0, 0.0], 5, Metric::Cosine, params)
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document.text, "rust doc");
    }

    cleanup(test_db);
}

// A collection indexed by dot product ranks by dot product.
#[test]
fn a_dot_product_collection_ranks_by_dot_product() {
    use piramid_core::config::{CollectionConfig, FlatConfig, IndexConfig};

    let path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_rank_by_dot.db");
    for suffix in ["", ".offsets.db", ".wal.db", ".vecindex.db", ".manifest.db"] {
        let _ = fs::remove_file(format!("{path}{suffix}"));
    }
    let config = CollectionConfig {
        index: IndexConfig::Flat {
            params: FlatConfig {
                metric: Metric::DotProduct,
                ..FlatConfig::default()
            },
        },
        ..CollectionConfig::default()
    };
    let mut collection = Collection::open_with_options(path, config.into()).unwrap();

    // Cosine ranks these near-identically; dot product orders them by magnitude.
    for (vector, text) in [
        (vec![1.0, 0.0], "unit"),
        (vec![10.0, 1.0], "large"),
        (vec![0.5, 0.0], "small"),
    ] {
        collection
            .insert(Document::new(vector, text.to_string()))
            .unwrap();
    }

    let hits = collection
        .search(&[1.0, 0.0], 3, Metric::DotProduct, SearchParams::default())
        .unwrap();

    assert_eq!(hits.len(), 3);
    assert_eq!(
        hits[0].document.text, "large",
        "highest dot product must rank first"
    );
    assert!(
        hits.windows(2).all(|w| w[0].score >= w[1].score),
        "scores must descend: {:?}",
        hits.iter().map(|h| h.score).collect::<Vec<_>>()
    );
}

// Searching by a metric other than the indexed one is refused.
#[test]
fn a_search_by_another_metric_than_the_index_is_refused() {
    use piramid_core::error::{ErrorKind, IndexError, PiramidError};

    let path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_metric_mismatch.db");
    for suffix in ["", ".offsets.db", ".wal.db", ".vecindex.db", ".manifest.db"] {
        let _ = fs::remove_file(format!("{path}{suffix}"));
    }
    let mut collection = Collection::open(path).unwrap();
    collection
        .insert(Document::new(vec![1.0, 0.0], "one".to_string()))
        .unwrap();
    assert_eq!(collection.vector_index().metric(), Metric::Cosine);

    let error = collection
        .search(&[1.0, 0.0], 1, Metric::DotProduct, SearchParams::default())
        .unwrap_err();
    assert!(matches!(
        error,
        PiramidError::Index(IndexError::MetricMismatch {
            indexed: Metric::Cosine,
            requested: Metric::DotProduct,
        })
    ));
    assert_eq!(error.kind(), ErrorKind::BadRequest);

    let error = collection
        .search_batch_with(
            &[vec![1.0, 0.0]],
            1,
            Metric::Euclidean,
            SearchParams::default(),
        )
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::BadRequest);
}

// A range query narrows the candidate set by threshold before k truncates it, so it returns every
// qualifying hit up to k.
#[test]
fn a_range_query_fills_k_from_the_whole_qualifying_set() {
    let path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_range_threshold.db");
    for suffix in ["", ".offsets.db", ".wal.db", ".vecindex.db", ".manifest.db"] {
        let _ = fs::remove_file(format!("{path}{suffix}"));
    }
    let mut collection = Collection::open(path).unwrap();

    // A shallow ramp: the first five clear 0.99, the rest do not.
    for i in 0..10 {
        let angle = i as f32 * 0.03;
        collection
            .insert(Document::new(
                vec![angle.cos(), angle.sin()],
                format!("doc{i}"),
            ))
            .unwrap();
    }

    let params = SearchParams {
        min_score: Some(0.99),
        ..SearchParams::default()
    };
    let hits = collection
        .search(&[1.0, 0.0], 3, Metric::Cosine, params)
        .unwrap();

    assert_eq!(hits.len(), 3, "k filled from everything that qualifies");
    for hit in &hits {
        assert!(
            hit.score >= 0.99,
            "{} scored {}",
            hit.document.text,
            hit.score
        );
    }
    assert!(hits.windows(2).all(|w| w[0].score >= w[1].score));
}

// A collection reopened under a configuration naming another metric refuses to open.
#[test]
fn reopening_under_another_metric_is_refused() {
    use piramid_core::config::{CollectionConfig, FlatConfig, IndexConfig};

    let path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_reopen_other_metric.db");
    for suffix in ["", ".offsets.db", ".wal.db", ".vecindex.db", ".manifest.db"] {
        let _ = fs::remove_file(format!("{path}{suffix}"));
    }
    let flat = |metric| CollectionConfig {
        index: IndexConfig::Flat {
            params: FlatConfig {
                metric,
                ..FlatConfig::default()
            },
        },
        ..CollectionConfig::default()
    };
    {
        let mut collection =
            Collection::open_with_options(path, flat(Metric::Cosine).into()).unwrap();
        collection
            .insert(Document::new(vec![1.0, 0.0], "one".to_string()))
            .unwrap();
        collection.checkpoint().unwrap();
    }
    let error = Collection::open_with_options(path, flat(Metric::DotProduct).into())
        .err()
        .expect("a metric change must not open silently");
    assert!(error.to_string().contains("indexed by cosine"), "{error}");
}

// A zero-magnitude vector is refused on every write and search of a cosine collection.
#[test]
fn a_zero_vector_is_refused_by_a_cosine_collection() {
    use piramid_core::error::ErrorKind;

    let path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_cosine_zero_vector.db");
    for suffix in ["", ".offsets.db", ".wal.db", ".vecindex.db", ".manifest.db"] {
        let _ = fs::remove_file(format!("{path}{suffix}"));
    }
    let mut collection = Collection::open(path).unwrap();
    assert_eq!(collection.vector_index().metric(), Metric::Cosine);
    let one = Document::new(vec![1.0, 0.0], "one".to_string());
    let one_id = one.id;
    collection.insert(one).unwrap();

    let zero = || Document::new(vec![0.0, 0.0], "zero".to_string());
    let error = collection.insert(zero()).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::BadRequest);
    assert!(error.to_string().contains("zero magnitude"), "{error}");
    assert_eq!(
        collection.insert_batch(vec![zero()]).unwrap_err().kind(),
        ErrorKind::BadRequest
    );
    let mut replacement = zero();
    replacement.id = one_id;
    assert_eq!(
        collection.upsert(replacement).unwrap_err().kind(),
        ErrorKind::BadRequest
    );
    assert_eq!(
        collection
            .search(&[0.0, 0.0], 1, Metric::Cosine, SearchParams::default())
            .unwrap_err()
            .kind(),
        ErrorKind::BadRequest
    );
    assert_eq!(
        collection
            .search_batch_with(
                &[vec![1.0, 0.0], vec![0.0, 0.0]],
                1,
                Metric::Cosine,
                SearchParams::default(),
            )
            .unwrap_err()
            .kind(),
        ErrorKind::BadRequest
    );
    let hits = collection
        .search(&[1.0, 0.0], 5, Metric::Cosine, SearchParams::default())
        .unwrap();
    assert_eq!(hits.len(), 1);
}

// A dot product collection scores a zero vector, so it accepts one.
#[test]
fn a_zero_vector_is_accepted_by_a_dot_product_collection() {
    use piramid_core::config::{CollectionConfig, FlatConfig, IndexConfig};

    let path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_dot_zero_vector.db");
    for suffix in ["", ".offsets.db", ".wal.db", ".vecindex.db", ".manifest.db"] {
        let _ = fs::remove_file(format!("{path}{suffix}"));
    }
    let config = CollectionConfig {
        index: IndexConfig::Flat {
            params: FlatConfig {
                metric: Metric::DotProduct,
                ..FlatConfig::default()
            },
        },
        ..CollectionConfig::default()
    };
    let mut collection = Collection::open_with_options(path, config.into()).unwrap();
    collection
        .insert(Document::new(vec![0.0, 0.0], "zero".to_string()))
        .unwrap();
    let hits = collection
        .search(&[0.0, 0.0], 1, Metric::DotProduct, SearchParams::default())
        .unwrap();
    assert_eq!(hits.len(), 1);
}

fn two_language_collection(path: &str) -> Collection {
    cleanup(path);
    let mut collection = Collection::open(path).unwrap();
    collection
        .insert(Document::with_metadata(
            vec![1.0, 0.0, 0.0],
            "python doc".to_string(),
            metadata([("lang", "python".into())]),
        ))
        .unwrap();
    collection
        .insert(Document::with_metadata(
            vec![0.9, 0.1, 0.0],
            "rust doc".to_string(),
            metadata([("lang", "rust".into())]),
        ))
        .unwrap();
    collection
}

// The filter overfetch of a per-query search config applies to a single query as it does to a batch.
#[test]
fn a_single_query_honours_the_overfetch_of_its_search_config() {
    let path = concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/test_search_overfetch_single.db"
    );
    let collection = two_language_collection(path);
    let filter = Filter::new().eq("lang", "rust");
    let mut search = collection.config().search;
    search.filter_overfetch = 1;
    let params = SearchParams {
        mode: collection.config().execution,
        filter: Some(&filter),
        search_config_override: Some(search),
        min_score: None,
    };

    let single = collection
        .search(&[1.0, 0.0, 0.0], 1, Metric::Cosine, params)
        .unwrap();
    let batch = collection
        .search_batch_with(&[vec![1.0, 0.0, 0.0]], 1, Metric::Cosine, params)
        .unwrap();

    assert!(single.is_empty(), "one candidate, filtered out");
    assert_eq!(single.len(), batch[0].len());
    drop(collection);
    cleanup(path);
}

#[test]
fn a_filter_overfetch_of_zero_is_refused() {
    let path = concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/test_search_overfetch_zero.db"
    );
    let collection = two_language_collection(path);
    let mut search = collection.config().search;
    search.filter_overfetch = 0;
    let params = SearchParams {
        search_config_override: Some(search),
        ..SearchParams::default()
    };

    let error = collection
        .search(&[1.0, 0.0, 0.0], 1, Metric::Cosine, params)
        .unwrap_err();

    assert!(
        error.to_string().contains("filter_overfetch must be >= 1"),
        "{error}"
    );
    drop(collection);
    cleanup(path);
}

// A duplicate scan with the default neighbour count finds the only pair of a two-document
// collection.
#[test]
fn a_duplicate_scan_of_two_documents_finds_their_pair() {
    let path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_duplicates_two.db");
    let collection = two_language_collection(path);

    let pairs =
        piramid_database::find_duplicates(&collection, Metric::Cosine, 0.9, None, None, None, None)
            .unwrap();

    assert_eq!(pairs.len(), 1);
    drop(collection);
    cleanup(path);
}

#[test]
fn a_duplicate_scan_refuses_zero_neighbours_ef_or_nprobe() {
    let path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_duplicates_zero.db");
    let collection = two_language_collection(path);

    for (k, ef, nprobe, name) in [
        (Some(0), None, None, "k"),
        (None, Some(0), None, "ef"),
        (None, None, Some(0), "nprobe"),
    ] {
        let error = piramid_database::find_duplicates(
            &collection,
            Metric::Cosine,
            0.9,
            None,
            k,
            ef,
            nprobe,
        )
        .unwrap_err();
        assert!(
            error.to_string().contains(&format!("{name} must be >= 1")),
            "{error}"
        );
    }
    drop(collection);
    cleanup(path);
}

// A document deleted from an HNSW collection is not reported as a duplicate.
#[test]
fn a_deleted_document_is_not_reported_as_a_duplicate() {
    use piramid_core::config::{CollectionConfig, HnswConfig, IndexConfig};

    let path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_duplicates_deleted.db");
    cleanup(path);
    let config = CollectionConfig {
        index: IndexConfig::Hnsw {
            params: HnswConfig::default(),
        },
        ..CollectionConfig::default()
    };
    let mut collection = Collection::open_with_options(path, config.into()).unwrap();
    let kept = collection
        .insert(Document::new(vec![1.0, 0.0, 0.0], "kept".to_string()))
        .unwrap();
    let deleted = collection
        .insert(Document::new(vec![1.0, 0.01, 0.0], "deleted".to_string()))
        .unwrap();
    collection
        .insert(Document::new(vec![0.0, 1.0, 0.0], "other".to_string()))
        .unwrap();
    assert!(collection.delete(&deleted).unwrap());

    let pairs =
        piramid_database::find_duplicates(&collection, Metric::Cosine, 0.9, None, None, None, None)
            .unwrap();

    assert!(
        pairs
            .iter()
            .all(|pair| pair.id_a != deleted && pair.id_b != deleted),
        "kept {kept} paired with deleted {deleted}: {pairs:?}"
    );
    drop(collection);
    cleanup(path);
}

#[test]
fn a_nan_score_is_not_ranked() {
    use piramid_core::Hit;
    use piramid_database::search::engine::rank_top_k;

    let hit = |score: f32, text: &str| Hit {
        score,
        document: Document::new(vec![1.0], text.to_string()),
    };
    let mut results = vec![hit(0.5, "half"), hit(f32::NAN, "nan"), hit(0.9, "high")];
    rank_top_k(&mut results, 3);
    let texts: Vec<&str> = results.iter().map(|h| h.document.text.as_str()).collect();
    assert_eq!(texts, ["high", "half"]);
}
