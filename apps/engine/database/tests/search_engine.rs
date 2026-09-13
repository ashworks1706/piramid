#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]
//! Search over a collection: exact scoring, filters, metrics and ranking.

use std::collections::HashMap;
use std::fs;
use {
    piramid_core::config::CollectionConfig,
    piramid_core::metadata::metadata,
    piramid_core::metadata::{Filter, Metadata},
    piramid_core::Document,
    piramid_database::resident::VectorStore,
    piramid_database::search::{SearchParams, SearchTarget},
    piramid_database::storage::{HashMapVectorReader, SidecarManager, VectorReader},
    piramid_database::{Collection, CollectionOpenOptions, ResidentManager},
    piramid_hardware::compute::strategies::for_mode,
    piramid_hardware::compute::{ExecutionMode, Metric},
    uuid::Uuid,
};

fn cleanup(path: &str) {
    let _ = fs::create_dir_all(env!("CARGO_TARGET_TMPDIR"));
    let _ = fs::remove_file(path);
    for sidecar in SidecarManager::at(path).all_paths() {
        let _ = fs::remove_file(&sidecar);
        let _ = fs::remove_file(format!("{sidecar}.tmp"));
    }
}

/// A fresh collection at path under metric.
fn collection_with_metric(path: &str, metric: Metric) -> Collection {
    cleanup(path);
    let mut config = CollectionConfig::default();
    config.search.metric = metric;
    Collection::open_with_options(path, CollectionOpenOptions { config }).unwrap()
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
        };

        let results = storage
            .search(&[1.0, 0.0, 0.0], 5, Metric::Cosine, params)
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document.text, "rust doc");
    }

    cleanup(test_db);
}

// A collection created with dot product ranks by dot product.
#[test]
fn a_dot_product_collection_ranks_by_dot_product() {
    let path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_rank_by_dot.db");
    let mut collection = collection_with_metric(path, Metric::DotProduct);

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

// Searching by a metric other than the one the collection was created with is refused.
#[test]
fn a_search_by_another_metric_than_the_collection_is_refused() {
    use piramid_core::error::{ErrorKind, PiramidError, SearchError};

    let path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_metric_mismatch.db");
    let mut collection = collection_with_metric(path, Metric::Cosine);
    collection
        .insert(Document::new(vec![1.0, 0.0], "one".to_string()))
        .unwrap();
    assert_eq!(collection.metric(), Metric::Cosine);

    let error = collection
        .search(&[1.0, 0.0], 1, Metric::DotProduct, SearchParams::default())
        .unwrap_err();
    assert!(matches!(
        error,
        PiramidError::Search(SearchError::MetricMismatch {
            collection: Metric::Cosine,
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

// A zero-magnitude vector is refused on every write and search of a cosine collection.
#[test]
fn a_zero_vector_is_refused_by_a_cosine_collection() {
    use piramid_core::error::ErrorKind;

    let path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_cosine_zero_vector.db");
    let mut collection = collection_with_metric(path, Metric::Cosine);
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
    let path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_dot_zero_vector.db");
    let mut collection = collection_with_metric(path, Metric::DotProduct);
    collection
        .insert(Document::new(vec![0.0, 0.0], "zero".to_string()))
        .unwrap();
    let hits = collection
        .search(&[0.0, 0.0], 1, Metric::DotProduct, SearchParams::default())
        .unwrap();
    assert_eq!(hits.len(), 1);
}

// A collection reopened under a configuration naming another metric keeps its own.
#[test]
fn reopening_under_another_metric_keeps_the_stored_metric() {
    let path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_reopen_other_metric.db");
    {
        let mut collection = collection_with_metric(path, Metric::Cosine);
        collection
            .insert(Document::new(vec![1.0, 0.0], "one".to_string()))
            .unwrap();
        collection.checkpoint().unwrap();
    }
    let mut config = CollectionConfig::default();
    config.search.metric = Metric::DotProduct;
    let collection = Collection::open_with_options(path, CollectionOpenOptions { config }).unwrap();
    assert_eq!(collection.metric(), Metric::Cosine);
    assert!(collection
        .search(&[1.0, 0.0], 1, Metric::DotProduct, SearchParams::default())
        .is_err());
    assert_eq!(
        collection
            .search(&[1.0, 0.0], 1, Metric::Cosine, SearchParams::default())
            .unwrap()
            .len(),
        1
    );
}

/// Documents on a half circle, where only every tenth carries lang rust, so the best-scoring
/// documents for a query along the x axis carry lang python.
fn ring_documents() -> Vec<Document> {
    (0..200)
        .map(|i| {
            let angle = i as f32 * std::f32::consts::PI / 200.0;
            let lang = if i % 10 == 9 { "rust" } else { "python" };
            Document::with_metadata(
                vec![angle.cos(), angle.sin()],
                format!("doc{i}"),
                metadata([("lang", lang.into())]),
            )
        })
        .collect()
}

// A filtered search returns k matches even when every best-scoring document fails the filter,
// before and after a delete leaves a hole in the slab.
#[test]
fn a_filtered_search_fills_k_when_the_best_documents_do_not_match() {
    let path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_filter_fills_k.db");
    let mut collection = collection_with_metric(path, Metric::Cosine);
    collection.insert_batch(ring_documents()).unwrap();
    let filter = Filter::new().eq("lang", "rust");
    let params = SearchParams {
        filter: Some(&filter),
        ..SearchParams::default()
    };
    let k = 7;

    let unfiltered = collection
        .search(&[1.0, 0.0], k, Metric::Cosine, SearchParams::default())
        .unwrap();
    assert!(unfiltered
        .iter()
        .all(|hit| hit.document.metadata["lang"] == "python".into()));

    assert!(collection.vector_reader().as_slab().is_some());
    let contiguous = collection
        .search(&[1.0, 0.0], k, Metric::Cosine, params)
        .unwrap();
    let texts: Vec<String> = contiguous
        .iter()
        .map(|hit| hit.document.text.clone())
        .collect();
    assert_eq!(
        texts,
        ["doc9", "doc19", "doc29", "doc39", "doc49", "doc59", "doc69"]
    );

    collection.delete(&unfiltered[0].document.id).unwrap();
    assert!(collection
        .vector_reader()
        .as_slab()
        .unwrap()
        .live
        .contains(&false));
    let gathered: Vec<String> = collection
        .search(&[1.0, 0.0], k, Metric::Cosine, params)
        .unwrap()
        .into_iter()
        .map(|hit| hit.document.text)
        .collect();
    assert_eq!(gathered, texts);
}

// A filter fewer than k documents match returns every match, and k of zero returns nothing.
#[test]
fn a_filter_matching_fewer_than_k_returns_every_match() {
    let path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_filter_fewer_than_k.db");
    let mut collection = collection_with_metric(path, Metric::Cosine);
    collection.insert_batch(ring_documents()).unwrap();
    let filter = Filter::new().eq("lang", "rust");
    let params = SearchParams {
        filter: Some(&filter),
        ..SearchParams::default()
    };

    let hits = collection
        .search(&[1.0, 0.0], 50, Metric::Cosine, params)
        .unwrap();

    assert_eq!(hits.len(), 20);
    assert!(hits.windows(2).all(|w| w[0].score >= w[1].score));
    assert!(collection
        .search(&[1.0, 0.0], 0, Metric::Cosine, params)
        .unwrap()
        .is_empty());
}

/// The contiguous slab, a scattered map and a store with a hole reach the kernel by different
/// paths, and every one of them ranks a collection as scoring one pair at a time does.
#[test]
fn every_scoring_path_ranks_a_collection_the_same_way() {
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
    let metadata: HashMap<Uuid, Metadata> = HashMap::new();
    let map: HashMap<Uuid, Vec<f32>> = rows.iter().cloned().collect();
    let resolve = |id: &Uuid| Ok(map.get(id).map(|vector| document_with_id(*id, vector)));

    let mut contiguous = ResidentManager::new();
    let mut holed = VectorStore::new();
    for (id, vector) in &rows {
        contiguous.put_vector(*id, vector).unwrap();
        holed.put(*id, vector).unwrap();
    }
    let evicted = Uuid::new_v4();
    holed.put(evicted, &vec![1.0; dim]).unwrap();
    holed.remove(&evicted);
    let scattered = HashMapVectorReader::new(&map);
    assert!(contiguous.as_slab().is_some());
    assert!(holed.as_slab().unwrap().live.contains(&false));
    assert!(scattered.as_slab().is_none());

    let k = 10;
    let params = SearchParams {
        mode: ExecutionMode::Scalar,
        filter: None,
    };
    for metric in [Metric::Cosine, Metric::Euclidean, Metric::DotProduct] {
        let ranked = |vectors: &dyn VectorReader| -> Vec<Uuid> {
            let target = SearchTarget {
                vectors,
                metadata: &metadata,
            };
            piramid_database::search::search(&target, &query, k, metric, params, &resolve)
                .unwrap()
                .into_iter()
                .map(|hit| hit.document.id)
                .collect()
        };
        let from_slab = ranked(&contiguous);
        assert_eq!(from_slab.len(), k);
        assert_eq!(from_slab, ranked(&scattered), "{metric:?}: slab vs gather");
        assert_eq!(from_slab, ranked(&holed), "{metric:?}: slab vs holed");

        let kernels = for_mode(ExecutionMode::Scalar).unwrap();
        let mut pairwise: Vec<(Uuid, f32)> = rows
            .iter()
            .map(|(id, vector)| (*id, metric.calculate(&query, vector, kernels).unwrap()))
            .collect();
        pairwise.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let expected: Vec<Uuid> = pairwise.into_iter().take(k).map(|(id, _)| id).collect();
        assert_eq!(from_slab, expected, "{metric:?}: batch vs pairwise");
    }
}

/// A document with the given id and vector and no text.
fn document_with_id(id: Uuid, vector: &[f32]) -> Document {
    let mut document = Document::new(vector.to_vec(), String::new());
    document.id = id;
    document
}

// A stored zero vector scores NaN under cosine and is never ranked, on the contiguous path and the
// gathered one.
#[test]
fn a_nan_score_is_not_ranked() {
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
    let metadata: HashMap<Uuid, Metadata> = HashMap::new();
    let resolve = |id: &Uuid| Ok(vectors.get(id).map(|vector| document_with_id(*id, vector)));
    let mut store = ResidentManager::new();
    for (id, vector) in &vectors {
        store.put_vector(*id, vector).unwrap();
    }
    let scattered = HashMapVectorReader::new(&vectors);

    for reader in [&store as &dyn VectorReader, &scattered] {
        let target = SearchTarget {
            vectors: reader,
            metadata: &metadata,
        };
        let hits = piramid_database::search::search(
            &target,
            &[1.0, 0.5, 0.25],
            31,
            Metric::Cosine,
            SearchParams::default(),
            &resolve,
        )
        .unwrap();
        assert_eq!(hits.len(), 30);
        assert!(hits.iter().all(|hit| hit.document.id != zero_id));
        assert!(hits.iter().all(|hit| !hit.score.is_nan()));
    }
}

#[test]
fn a_query_of_another_width_is_refused() {
    let path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_query_width.db");
    let mut collection = collection_with_metric(path, Metric::Cosine);
    collection
        .insert(Document::new(vec![1.0, 0.0], "one".to_string()))
        .unwrap();

    let error = collection
        .search(&[1.0, 0.0, 0.0], 1, Metric::Cosine, SearchParams::default())
        .unwrap_err();

    assert!(error.to_string().contains("dimension mismatch"), "{error}");
}
