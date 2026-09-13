#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]
//! Collection handling through the serving layer.

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
use piramid_serving::services::api::{InsertRequest, ListVectorsQuery, SearchRequest};
use piramid_serving::services::collection::record_rebuild_panic;
use piramid_serving::state::{AppState, RebuildJobStatus, RebuildState};
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

// allow-panic-in-tests does not cover a function outside a #[test].
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
            ef: None,
            nprobe: None,
            filter_overfetch: None,
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

#[test]
fn a_collection_name_that_is_not_a_plain_name_is_refused_by_the_manager() {
    let data_dir = concat!(env!("CARGO_TARGET_TMPDIR"), "/collection_manager_names");
    let state = test_state(data_dir);
    for name in ["../outside", "a/b", "", "dot.name"] {
        let manager = &state.collection_manager;
        assert_eq!(
            manager.get_existing(name).err().unwrap().kind(),
            ErrorKind::BadRequest,
            "{name}"
        );
        assert_eq!(
            manager.get_or_create(name).err().unwrap().kind(),
            ErrorKind::BadRequest,
            "{name}"
        );
        assert_eq!(
            manager.delete(name).unwrap_err().kind(),
            ErrorKind::BadRequest,
            "{name}"
        );
    }
    cleanup_dir(data_dir);
}

// A collection on disk that is not open is deleted with its files, and one that exists nowhere
// is not found.
#[test]
fn deleting_removes_a_collection_that_is_only_on_disk() {
    let data_dir = concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/collection_manager_delete_on_disk"
    );
    let state = test_state(data_dir);
    {
        let handle = state.collection_manager.get_or_create("docs").unwrap();
        let mut guard = handle.write();
        guard
            .insert(Document::new(vec![1.0, 0.0], "one".to_string()))
            .unwrap();
        guard.checkpoint().unwrap();
    }
    // A fresh state has nothing open.
    let mut config = Config::default();
    config.startup.data_dir = data_dir.to_string();
    let fresh = AppState::new(
        config,
        piramid_model::embeddings::EmbeddingsManager::disabled(),
    )
    .unwrap();
    assert!(!fresh.collection_manager.contains_loaded("docs"));
    assert_eq!(
        fresh.collection_manager.discover_on_disk().unwrap(),
        vec!["docs".to_string()]
    );

    fresh.collection_manager.delete("docs").unwrap();
    assert!(fresh
        .collection_manager
        .discover_on_disk()
        .unwrap()
        .is_empty());
    assert!(
        fs::read_dir(data_dir).unwrap().next().is_none(),
        "no sidecar left"
    );
    assert_eq!(
        fresh.collection_manager.delete("docs").unwrap_err().kind(),
        ErrorKind::NotFound
    );
    cleanup_dir(data_dir);
}

#[test]
fn read_only_lifts_once_there_is_space_again() {
    use std::sync::atomic::Ordering;

    let data_dir = concat!(env!("CARGO_TARGET_TMPDIR"), "/collection_manager_read_only");
    let mut config = Config::default();
    config.startup.disk.min_free_bytes = Some(0);
    config.startup.disk.readonly_on_low_space = true;
    let state = test_state_with_config(data_dir, config);

    state.read_only.store(true, Ordering::Relaxed);
    state.ensure_write_allowed().unwrap();
    assert!(!state.read_only.load(Ordering::Relaxed));
    cleanup_dir(data_dir);
}

#[test]
fn a_write_below_the_disk_floor_fails_without_read_only() {
    use std::sync::atomic::Ordering;

    let data_dir = concat!(env!("CARGO_TARGET_TMPDIR"), "/collection_manager_low_disk");
    let mut config = Config::default();
    config.startup.disk.min_free_bytes = Some(u64::MAX);
    config.startup.disk.readonly_on_low_space = false;
    let state = test_state_with_config(data_dir, config);

    let error = state.ensure_write_allowed().unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Unavailable);
    assert!(!state.read_only.load(Ordering::Relaxed));
    cleanup_dir(data_dir);
}

#[tokio::test]
async fn a_rebuild_while_one_is_running_is_a_conflict() {
    use piramid_serving::state::{RebuildJobStatus, RebuildState};

    let data_dir = concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/collection_manager_rebuild_conflict"
    );
    let state = test_state(data_dir);
    state.collection_manager.get_or_create("docs").unwrap();
    state.rebuild_jobs.insert(
        "docs".to_string(),
        RebuildJobStatus {
            status: RebuildState::Running,
            started_at: 0,
            finished_at: None,
            error: None,
            elapsed_ms: None,
        },
    );
    let error = piramid_serving::services::collection::rebuild_index(&state, "docs".to_string())
        .err()
        .unwrap();
    assert_eq!(error.kind(), ErrorKind::Conflict);
    cleanup_dir(data_dir);
}

/// Reports a token count for texts that start with a digit and none for the rest.
struct CountsSome;

#[async_trait::async_trait]
impl piramid_model::embeddings::Embedder for CountsSome {
    async fn embed(
        &self,
        text: &str,
    ) -> piramid_model::embeddings::EmbeddingResult<piramid_model::embeddings::EmbeddingResponse>
    {
        Ok(piramid_model::embeddings::EmbeddingResponse {
            embedding: vec![1.0, 0.0, 0.0],
            tokens: text.starts_with(char::is_numeric).then_some(3),
            model: "counts-some".to_string(),
        })
    }

    fn provider_name(&self) -> &'static str {
        "counts-some"
    }

    fn model_name(&self) -> &str {
        "counts-some"
    }
}

#[tokio::test]
async fn embed_total_tokens_is_absent_when_any_text_went_uncounted() {
    use piramid_serving::services::api::EmbedRequest;
    use piramid_serving::services::embedding::embed_text;

    let data_dir = concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/collection_manager_embed_tokens"
    );
    cleanup_dir(data_dir);
    let mut config = Config::default();
    config.startup.data_dir = data_dir.to_string();
    let state = Arc::new(
        AppState::new(
            config,
            piramid_model::embeddings::EmbeddingsManager::with_embedder(Arc::new(CountsSome)),
        )
        .unwrap(),
    );
    let request = |texts: &[&str]| EmbedRequest {
        texts: texts.iter().map(|text| text.to_string()).collect(),
        metadata: Vec::new(),
    };

    let counted = embed_text(&state, "docs".to_string(), request(&["1 one", "2 two"]))
        .await
        .unwrap();
    assert_eq!(counted.total_tokens, Some(6));

    let mixed = embed_text(&state, "docs".to_string(), request(&["1 one", "two"]))
        .await
        .unwrap();
    assert_eq!(mixed.total_tokens, None);
    cleanup_dir(data_dir);
}

#[tokio::test]
async fn a_rebuild_that_panics_is_recorded_as_failed() {
    let jobs = Arc::new(dashmap::DashMap::new());
    jobs.insert(
        "docs".to_string(),
        RebuildJobStatus {
            status: RebuildState::Running,
            started_at: 7,
            finished_at: None,
            error: None,
            elapsed_ms: None,
        },
    );
    let rebuild = tokio::task::spawn_blocking(|| panic!("index out of bounds"));
    record_rebuild_panic(
        rebuild,
        jobs.clone(),
        "docs".to_string(),
        7,
        std::time::Instant::now(),
    )
    .await;

    let job = jobs.get("docs").unwrap();
    assert_eq!(job.status, RebuildState::Failed);
    assert_eq!(job.started_at, 7);
    assert!(job.finished_at.is_some());
    assert!(
        job.error.as_deref().unwrap().contains("panic"),
        "{:?}",
        job.error
    );
}
