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
    let normalized = validation::normalize_vector(&vec);
    let magnitude: f32 = normalized.iter().map(|&x| x * x).sum::<f32>().sqrt();
    assert!((magnitude - 1.0).abs() < 0.0001);

    let zero = vec![0.0, 0.0];
    assert_eq!(validation::normalize_vector(&zero), zero);
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
fn tuning_fields_match_the_config_fields_they_override() {
    let json = serde_json::json!({ "ef": 5, "nprobe": 6, "filter_overfetch": 7 });
    let tuning: piramid_serving::services::api::SearchTuning =
        serde_json::from_value(json).unwrap();

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
