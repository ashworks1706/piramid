#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]

use axum::{
    extract::{Path, State},
    Json,
};
use piramid_core::config::Config;
use piramid_core::error::{ErrorKind, PiramidError};
use piramid_core::metadata::metadata;
use piramid_core::Document;
use piramid_database::Collection;
use piramid_serving::http::handlers::{collections, vectors};
use piramid_serving::http::ApiResult;
use piramid_serving::services::api::{
    InsertRequest, ListVectorsQuery, SearchRequest, SearchTuning,
};
use piramid_serving::state::AppState;
use std::{fs, sync::Arc};

fn cleanup_dir(path: &str) {
    let _ = fs::remove_dir_all(path);
}

fn test_state(data_dir: &str) -> Arc<AppState> {
    test_state_with_config(data_dir, Config::default())
}

fn test_state_with_config(data_dir: &str, mut config: Config) -> Arc<AppState> {
    cleanup_dir(data_dir);
    config.startup.data_dir = data_dir.to_string();
    Arc::new(
        AppState::new(
            config,
            piramid_model::embeddings::EmbeddingsManager::disabled(),
        )
        .unwrap(),
    )
}

// Not a #[test] itself, so allow-panic-in-tests does not cover it.
#[allow(clippy::panic)]
fn assert_not_found<T>(result: ApiResult<T>) {
    match result {
        Err(error) => {
            assert!(
                matches!(error.0, PiramidError::Server(_)),
                "expected a server error, got {:?}",
                error.0
            );
            assert_eq!(error.0.kind(), ErrorKind::NotFound);
        }
        Ok(_) => panic!("expected not-found error"),
    }
}

#[tokio::test]
async fn read_endpoints_do_not_create_missing_collections() {
    let data_dir = concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/collection_manager_missing_reads"
    );
    let state = test_state(data_dir);

    assert_not_found(
        collections::get_collection(State(state.clone()), Path("missing".to_string())).await,
    );
    assert_not_found(
        vectors::list_vectors(
            State(state.clone()),
            Path("missing".to_string()),
            axum::extract::Query(ListVectorsQuery {
                limit: 10,
                offset: 0,
            }),
        )
        .await,
    );

    assert_eq!(state.collection_manager.len(), 0);
    assert!(!std::path::Path::new(&format!("{data_dir}/missing.db")).exists());

    cleanup_dir(data_dir);
}

#[tokio::test]
async fn cache_budget_evicts_metadata_without_dropping_vectors() {
    let data_dir = concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/collection_manager_cache_budget"
    );
    let mut app_config = Config::default();
    app_config.runtime.cache.metadata.max_bytes = Some(1);
    let state = test_state_with_config(data_dir, app_config);
    let collection = state
        .collection_manager
        .get_or_create("docs")
        .expect("create collection");

    {
        let mut collection_guard = collection.write();
        collection_guard
            .insert(Document::with_metadata(
                vec![1.0, 0.0, 0.0],
                "first".to_string(),
                metadata([("kind", "a".into())]),
            ))
            .unwrap();
        collection_guard
            .insert(Document::with_metadata(
                vec![0.0, 1.0, 0.0],
                "second".to_string(),
                metadata([("kind", "b".into())]),
            ))
            .unwrap();
        assert_eq!(collection_guard.vector_reader().len(), 2);
        assert_eq!(collection_guard.metadata_view().len(), 2);
    }

    state.enforce_cache_budget();

    {
        let collection_guard = collection.read();
        assert_eq!(collection_guard.vector_reader().len(), 2);
        assert_eq!(collection_guard.metadata_view().len(), 0);
        assert_eq!(collection_guard.count(), 2);
    }

    cleanup_dir(data_dir);
}

#[tokio::test]
async fn insert_endpoint_creates_collection_intentionally() {
    let data_dir = concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/collection_manager_insert_creates"
    );
    let state = test_state(data_dir);

    let response = vectors::insert_vector(
        State(state.clone()),
        Path("docs".to_string()),
        Json(InsertRequest {
            vectors: vec![vec![1.0, 0.0, 0.0]],
            texts: vec!["created by insert".to_string()],
            metadata: Vec::new(),
            normalize: false,
        }),
    )
    .await
    .expect("insert should create collection");

    assert_eq!(response.0.count, 1);
    assert!(!response.0.ids[0].is_empty());

    assert_eq!(state.collection_manager.len(), 1);
    assert!(std::path::Path::new(&format!("{data_dir}/docs.db")).exists());

    cleanup_dir(data_dir);
}

#[tokio::test]
async fn read_endpoint_loads_existing_collection_from_disk() {
    let data_dir = concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/collection_manager_existing_disk"
    );
    let collection_path = format!("{data_dir}/docs.db");
    let state = test_state(data_dir);
    fs::create_dir_all(data_dir).expect("create test data dir");

    {
        let mut collection = Collection::open(&collection_path).expect("create collection");
        collection
            .insert(Document::new(vec![1.0, 0.0, 0.0], "stored doc".to_string()))
            .expect("insert document");
        collection.checkpoint().expect("checkpoint collection");
    }

    let response = collections::get_collection(State(state.clone()), Path("docs".to_string()))
        .await
        .expect("existing collection should load");

    assert_eq!(response.0.name, "docs");
    assert_eq!(response.0.count, 1);
    assert_eq!(state.collection_manager.len(), 1);

    cleanup_dir(data_dir);
}

#[tokio::test]
async fn search_applies_a_metadata_filter_from_the_request() {
    let data_dir = concat!(env!("CARGO_TARGET_TMPDIR"), "/collection_manager_filter");
    let state = test_state(data_dir);

    let _ = vectors::insert_vector(
        State(state.clone()),
        Path("docs".to_string()),
        Json(InsertRequest {
            vectors: vec![vec![1.0, 0.0], vec![0.9, 0.1], vec![0.8, 0.2]],
            texts: vec!["a".into(), "b".into(), "c".into()],
            metadata: vec![
                [("lang".to_string(), serde_json::json!("rust"))].into(),
                [("lang".to_string(), serde_json::json!("go"))].into(),
                [("lang".to_string(), serde_json::json!("rust"))].into(),
            ],
            normalize: false,
        }),
    )
    .await
    .expect("insert should succeed");

    let mut ops = std::collections::HashMap::new();
    ops.insert("eq".to_string(), serde_json::json!("rust"));
    let mut filter = std::collections::HashMap::new();
    filter.insert("lang".to_string(), ops);

    let response = vectors::search_vectors(
        State(state.clone()),
        Path("docs".to_string()),
        axum::Extension(piramid_serving::http::request_id::RequestId("test".into())),
        Json(SearchRequest {
            vectors: vec![vec![1.0, 0.0]],
            k: 10,
            metric: None,
            filter: Some(filter),
            tuning: SearchTuning::default(),
        }),
    )
    .await
    .expect("search should succeed");

    let hits = &response.0.results[0];
    assert_eq!(hits.len(), 2, "only the two rust documents should survive");
    for hit in hits {
        assert_eq!(hit.metadata["lang"], serde_json::json!("rust"));
    }
}
