use piramid::config::ExecutionMode;
use piramid::metrics::{
    cosine::cosine_similarity,
    dot_product,
    euclidean::{euclidean_distance, euclidean_distance_squared},
    Metric,
};

#[test]
fn euclidean_distance_basic_cases() {
    let v = vec![1.0, 2.0, 3.0];
    assert_eq!(euclidean_distance(&v, &v, ExecutionMode::Auto), 0.0);

    let v1 = vec![0.0, 0.0];
    let v2 = vec![3.0, 4.0];
    let dist = euclidean_distance(&v1, &v2, ExecutionMode::Auto);
    assert!((dist - 5.0).abs() < 1e-6);

    let sq = euclidean_distance_squared(&v1, &v2, ExecutionMode::Auto);
    assert!((sq - 25.0).abs() < 1e-6);
}

#[test]
#[should_panic(expected = "Vectors must have same length")]
fn euclidean_rejects_mismatched_lengths() {
    euclidean_distance(&[1.0, 2.0], &[1.0], ExecutionMode::Auto);
}

#[test]
fn dot_product_basic_cases() {
    let v1 = vec![1.0, 2.0, 3.0];
    let v2 = vec![4.0, 5.0, 6.0];
    let result = dot_product(&v1, &v2, ExecutionMode::Auto);
    assert!((result - 32.0).abs() < 1e-6);

    let ortho = dot_product(&[1.0, 0.0], &[0.0, 1.0], ExecutionMode::Auto);
    assert!(ortho.abs() < 1e-6);
}

#[test]
#[should_panic(expected = "Vectors must have same length")]
fn dot_rejects_mismatched_lengths() {
    dot_product(&[1.0, 2.0], &[1.0], ExecutionMode::Auto);
}

#[test]
fn metric_calculate_cosine_and_euclidean() {
    let v1 = vec![1.0, 0.0];
    let v2 = vec![0.0, 1.0];
    assert!(
        Metric::Cosine
            .calculate(&v1, &v2, ExecutionMode::Auto)
            .abs()
            < 1e-6
    );

    let euclid_sim = Metric::Euclidean.calculate(&v1, &v1, ExecutionMode::Auto);
    assert!((euclid_sim - 1.0).abs() < 1e-6);
}

#[test]
fn cosine_similarity_cases() {
    let v = vec![1.0, 2.0, 3.0];
    let sim_same = cosine_similarity(&v, &v, ExecutionMode::Auto);
    assert!((sim_same - 1.0).abs() < 1e-6);

    let sim_orth = cosine_similarity(&[1.0, 0.0], &[0.0, 1.0], ExecutionMode::Auto);
    assert!(sim_orth.abs() < 1e-6);
}
