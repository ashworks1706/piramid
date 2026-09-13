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
            filter_overfetch_override: None,
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
