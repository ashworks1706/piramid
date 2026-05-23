use axum::{
    extract::{Path, State},
    Json,
};
use piramid::{
    config::AppConfig,
    error::PiramidError,
    server::{
        handlers::{collections, vectors},
        types::{InsertRequest, InsertResultsResponse, ListVectorsQuery},
        AppState,
    },
    Collection, Document,
};
use std::{collections::HashMap, fs, sync::Arc};

fn cleanup_dir(path: &str) {
    let _ = fs::remove_dir_all(path);
}

fn test_state(data_dir: &str) -> Arc<AppState> {
    cleanup_dir(data_dir);
    Arc::new(AppState::new(
        data_dir,
        AppConfig::default(),
        500,
        None,
        true,
    ))
}

fn assert_not_found<T>(result: piramid::Result<T>) {
    match result {
        Err(PiramidError::Server(error)) => {
            assert_eq!(error.status_code(), axum::http::StatusCode::NOT_FOUND);
        }
        Err(error) => panic!("expected server not-found error, got {error:?}"),
        Ok(_) => panic!("expected not-found error"),
    }
}

#[tokio::test]
async fn read_endpoints_do_not_create_missing_collections() {
    let data_dir = ".piramid/tests/server_registry_missing_reads";
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

    assert_eq!(state.registry.len(), 0);
    assert!(!std::path::Path::new(&format!("{data_dir}/missing.db")).exists());

    cleanup_dir(data_dir);
}

#[tokio::test]
async fn insert_endpoint_creates_collection_intentionally() {
    let data_dir = ".piramid/tests/server_registry_insert_creates";
    let state = test_state(data_dir);

    let response = vectors::insert_vector(
        State(state.clone()),
        Path("docs".to_string()),
        Json(InsertRequest {
            vector: Some(vec![1.0, 0.0, 0.0]),
            vectors: None,
            text: Some("created by insert".to_string()),
            texts: None,
            metadata: HashMap::new(),
            metadata_list: Vec::new(),
            normalize: false,
        }),
    )
    .await
    .expect("insert should create collection");

    match response.0 {
        InsertResultsResponse::Single(single) => assert!(!single.id.is_empty()),
        InsertResultsResponse::Multi(_) => panic!("expected single insert response"),
    }

    assert_eq!(state.registry.len(), 1);
    assert!(std::path::Path::new(&format!("{data_dir}/docs.db")).exists());

    cleanup_dir(data_dir);
}

#[tokio::test]
async fn read_endpoint_loads_existing_collection_from_disk() {
    let data_dir = ".piramid/tests/server_registry_existing_disk";
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

    let response =
        collections::get_collection(State(state.clone()), Path("docs".to_string()))
            .await
            .expect("existing collection should load");

    assert_eq!(response.0.name, "docs");
    assert_eq!(response.0.count, 1);
    assert_eq!(state.registry.len(), 1);

    cleanup_dir(data_dir);
}
