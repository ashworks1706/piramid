use piramid::validation;

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
fn invalid_metric_and_search_preset_are_rejected() {
    assert!(piramid::services::search::parse_metric(Some("cosinee".into())).is_err());
    assert!(piramid::services::search::apply_search_overrides(
        piramid::config::SearchConfig::default(),
        None,
        None,
        None,
        Some("balanced".into()),
    )
    .is_err());
}
