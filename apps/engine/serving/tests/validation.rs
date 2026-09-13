#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]
//! Request validation and conversion.

use piramid_core::validation;

#[test]
fn validate_vector_cases() {
    assert!(validation::validate_vector(&[1.0, 2.0, 3.0]).is_ok());
    assert!(validation::validate_vector(&[0.0, -1.5, 100.0]).is_ok());
    assert!(validation::validate_vector(&[]).is_err());
    assert!(validation::validate_vector(&[1.0, f32::NAN]).is_err());
    assert!(validation::validate_vector(&[1.0, f32::INFINITY]).is_err());
}

#[test]
fn normalize_vector_behaviour() {
    let vec = vec![3.0, 4.0];
    let normalized = validation::normalize_vector(&vec).unwrap();
    let magnitude: f32 = normalized.iter().map(|&x| x * x).sum::<f32>().sqrt();
    assert!((magnitude - 1.0).abs() < 0.0001);

    let error = validation::normalize_vector(&[0.0, 0.0]).unwrap_err();
    assert!(
        error.to_string().contains("cannot be normalized"),
        "{error}"
    );
    assert!(validation::normalize_vector(&[f32::MAX, f32::MAX]).is_err());
}

#[test]
fn validate_dimensions_and_names() {
    assert!(validation::validate_dimensions(&[1.0, 2.0, 3.0], 3).is_ok());
    assert!(validation::validate_dimensions(&[1.0, 2.0], 3).is_err());

    assert!(validation::validate_collection_name("my_collection-1").is_ok());
    assert!(validation::validate_collection_name("").is_err());
    assert!(validation::validate_collection_name("bad name").is_err());
}

#[test]
fn validate_batch_sizes() {
    assert!(validation::validate_batch_size(10, 100, "insert").is_ok());
    assert!(validation::validate_batch_size(0, 100, "insert").is_err());
    assert!(validation::validate_batch_size(101, 100, "insert").is_err());
}

#[test]
fn invalid_metric_is_rejected() {
    use piramid_hardware::compute::Metric;
    use piramid_serving::services::convert::parse_metric;

    assert!(parse_metric(Some("cosinee".into()), Metric::Cosine).is_err());
    assert!(parse_metric(Some("dot_product".into()), Metric::Cosine).is_err());
    assert_eq!(
        parse_metric(Some("dot".into()), Metric::Cosine).unwrap(),
        Metric::DotProduct
    );
}

#[test]
fn an_absent_metric_is_the_indexed_metric() {
    use piramid_hardware::compute::Metric;
    use piramid_serving::services::convert::parse_metric;

    for indexed in [Metric::Cosine, Metric::Euclidean, Metric::DotProduct] {
        assert_eq!(parse_metric(None, indexed).unwrap(), indexed);
    }
}

#[test]
fn zero_valued_tuning_knobs_are_rejected() {
    use piramid_serving::services::api::SearchTuning;
    let base = piramid_core::config::SearchConfig::default();
    for tuning in [
        SearchTuning {
            ef: Some(0),
            ..Default::default()
        },
        SearchTuning {
            nprobe: Some(0),
            ..Default::default()
        },
        SearchTuning {
            filter_overfetch: Some(0),
            ..Default::default()
        },
    ] {
        assert!(piramid_serving::services::convert::apply_search_overrides(base, &tuning).is_err());
    }
}

#[test]
fn unknown_filter_operators_are_rejected() {
    use std::collections::HashMap;
    let mut ops = HashMap::new();
    ops.insert("between".to_string(), serde_json::json!(3));
    let mut raw = HashMap::new();
    raw.insert("year".to_string(), ops);
    assert!(piramid_serving::services::convert::parse_filter(Some(raw)).is_err());
}

#[test]
fn a_range_filter_with_a_non_numeric_value_is_rejected() {
    use std::collections::HashMap;
    let parse = |op: &str, value: serde_json::Value| {
        let mut ops = HashMap::new();
        ops.insert(op.to_string(), value);
        let mut raw = HashMap::new();
        raw.insert("date".to_string(), ops);
        piramid_serving::services::convert::parse_filter(Some(raw))
    };
    for op in ["gt", "gte", "lt", "lte"] {
        let error = parse(op, serde_json::json!("2024-01-01")).unwrap_err();
        assert!(error.to_string().contains("expects a number"), "{error}");
        assert!(parse(op, serde_json::json!(true)).is_err());
        assert!(parse(op, serde_json::json!(3)).is_ok());
        assert!(parse(op, serde_json::json!(2.5)).is_ok());
    }
    assert!(parse("eq", serde_json::json!("2024-01-01")).is_ok());
}

#[test]
fn tuning_fields_match_the_config_fields_they_override() {
    let json = serde_json::json!({
        "vectors": [[1.0]], "ef": 5, "nprobe": 6, "filter_overfetch": 7
    });
    let request: piramid_serving::services::api::SearchRequest =
        serde_json::from_value(json).unwrap();
    let tuning = piramid_serving::services::api::SearchTuning {
        ef: request.ef,
        nprobe: request.nprobe,
        filter_overfetch: request.filter_overfetch,
    };

    let applied = piramid_serving::services::convert::apply_search_overrides(
        piramid_core::config::SearchConfig::default(),
        &tuning,
    )
    .unwrap();

    assert_eq!(applied.ef, Some(5));
    assert_eq!(applied.nprobe, Some(6));
    assert_eq!(applied.filter_overfetch, 7);
}

#[test]
fn a_rejected_tuning_value_names_the_field_the_user_wrote() {
    let tuning = piramid_serving::services::api::SearchTuning {
        filter_overfetch: Some(0),
        ..Default::default()
    };
    let error = piramid_serving::services::convert::apply_search_overrides(
        piramid_core::config::SearchConfig::default(),
        &tuning,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("filter_overfetch"), "{error}");
}

#[test]
fn request_shapes_refuse_unknown_fields() {
    use piramid_serving::services::api::{
        CreateCollectionRequest, DeleteVectorsRequest, DuplicateRequest, EmbedRequest,
        InsertRequest, ListVectorsQuery, RangeSearchRequest, SearchRequest, TextSearchRequest,
        UpsertRequest,
    };
    use serde_json::json;

    fn refused<T: serde::de::DeserializeOwned>(value: serde_json::Value) {
        let error = serde_json::from_value::<T>(value)
            .err()
            .expect("an unknown field is refused");
        assert!(error.to_string().contains("unknown field"), "{error}");
    }
    fn accepted<T: serde::de::DeserializeOwned>(value: serde_json::Value) {
        serde_json::from_value::<T>(value).unwrap();
    }

    accepted::<SearchRequest>(json!({"vectors": [[1.0]], "k": 3, "ef": 8, "filter_overfetch": 2}));
    refused::<SearchRequest>(json!({"vectors": [[1.0]], "top_k": 3}));
    accepted::<RangeSearchRequest>(json!({"vectors": [[1.0]], "min_score": 0.5, "nprobe": 2}));
    refused::<RangeSearchRequest>(json!({"vectors": [[1.0]], "min_score": 0.5, "efSearch": 2}));
    accepted::<TextSearchRequest>(json!({"query": "q", "ef": 8}));
    refused::<TextSearchRequest>(json!({"query": "q", "limit": 3}));
    refused::<EmbedRequest>(json!({"texts": ["a"], "metadatas": [{}]}));
    refused::<InsertRequest>(json!({"vectors": [[1.0]], "texts": ["a"], "vector": [1.0]}));
    refused::<ListVectorsQuery>(json!({"page": 2}));
    refused::<DeleteVectorsRequest>(json!({"ids": [], "id": "x"}));
    refused::<UpsertRequest>(json!({"vector": [1.0], "text": "a", "normalise": true}));
    refused::<DuplicateRequest>(json!({"min_score": 0.9}));
    refused::<CreateCollectionRequest>(json!({"name": "docs", "dimensions": 3}));
}

#[test]
fn a_list_query_string_with_an_unknown_parameter_is_refused() {
    use axum::extract::Query;
    use piramid_serving::services::api::ListVectorsQuery;

    let uri: axum::http::Uri = "/vectors?limit=5&page=2".parse().unwrap();
    assert!(Query::<ListVectorsQuery>::try_from_uri(&uri).is_err());
    let uri: axum::http::Uri = "/vectors?limit=5&offset=2".parse().unwrap();
    let query = Query::<ListVectorsQuery>::try_from_uri(&uri).unwrap();
    assert_eq!((query.limit, query.offset), (5, 2));
}
