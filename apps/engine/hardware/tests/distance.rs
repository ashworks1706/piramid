#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]
//! Distance kernels, metrics and the parity of every strategy with scalar.

use piramid_hardware::compute::strategies::for_mode;
use piramid_hardware::compute::{
    cosine_similarity, dot_product, euclidean_distance, euclidean_distance_squared, ComputeError,
    DistanceKernels, ExecutionMode, Metric,
};

fn auto() -> &'static dyn DistanceKernels {
    for_mode(ExecutionMode::Auto).expect("Auto always resolves to an available CPU strategy")
}

#[test]
fn euclidean_distance_basic_cases() {
    let v = vec![1.0, 2.0, 3.0];
    assert_eq!(euclidean_distance(&v, &v, auto()).unwrap(), 0.0);

    let v1 = vec![0.0, 0.0];
    let v2 = vec![3.0, 4.0];
    let dist = euclidean_distance(&v1, &v2, auto()).unwrap();
    assert!((dist - 5.0).abs() < 1e-6);

    let sq = euclidean_distance_squared(&v1, &v2, auto()).unwrap();
    assert!((sq - 25.0).abs() < 1e-6);
}

#[test]
fn euclidean_rejects_mismatched_lengths() {
    let error = euclidean_distance(&[1.0, 2.0], &[1.0], auto()).unwrap_err();
    assert!(matches!(error, ComputeError::ShapeMismatch { .. }));
}

#[test]
fn dot_product_basic_cases() {
    let v1 = vec![1.0, 2.0, 3.0];
    let v2 = vec![4.0, 5.0, 6.0];
    let result = dot_product(&v1, &v2, auto()).unwrap();
    assert!((result - 32.0).abs() < 1e-6);

    let ortho = dot_product(&[1.0, 0.0], &[0.0, 1.0], auto()).unwrap();
    assert!(ortho.abs() < 1e-6);
}

#[test]
fn dot_rejects_mismatched_lengths() {
    let error = dot_product(&[1.0, 2.0], &[1.0], auto()).unwrap_err();
    assert!(matches!(error, ComputeError::ShapeMismatch { .. }));
}

/// Every available strategy refuses a pair of different lengths on every pairwise kernel.
#[test]
fn every_pairwise_kernel_rejects_mismatched_lengths() {
    use piramid_hardware::compute::strategies::all;

    for kernels in all().into_iter().filter(|k| k.is_available()) {
        let (a, b) = ([1.0, 2.0, 3.0], [1.0, 2.0]);
        assert!(kernels.cosine(&a, &b).is_err(), "{}", kernels.name());
        assert!(kernels.dot(&a, &b).is_err(), "{}", kernels.name());
        assert!(kernels.euclidean(&a, &b).is_err(), "{}", kernels.name());
        assert!(
            kernels.euclidean_squared(&a, &b).is_err(),
            "{}",
            kernels.name()
        );
    }
}

/// Cosine against a zero vector is NaN, not a score, on every available strategy.
#[test]
fn cosine_against_a_zero_vector_is_nan() {
    use piramid_hardware::compute::strategies::all;

    for kernels in all().into_iter().filter(|k| k.is_available()) {
        let query = [1.0, 2.0, 3.0];
        let zero = [0.0; 3];
        assert!(
            kernels.cosine(&query, &zero).unwrap().is_nan(),
            "{}",
            kernels.name()
        );
        assert!(
            kernels.cosine(&zero, &query).unwrap().is_nan(),
            "{}",
            kernels.name()
        );
        let mut out = [0.0; 2];
        let slab = [0.0, 0.0, 0.0, 1.0, 2.0, 3.0];
        kernels.cosine_batch(&query, &slab, 3, &mut out).unwrap();
        assert!(out[0].is_nan(), "{}", kernels.name());
        assert!((out[1] - 1.0).abs() < 1e-6, "{}", kernels.name());
    }
}

#[test]
fn metric_calculate_cosine_and_euclidean() {
    let v1 = vec![1.0, 0.0];
    let v2 = vec![0.0, 1.0];
    assert!(Metric::Cosine.calculate(&v1, &v2, auto()).unwrap().abs() < 1e-6);

    let euclid_sim = Metric::Euclidean.calculate(&v1, &v1, auto()).unwrap();
    assert!((euclid_sim - 1.0).abs() < 1e-6);
}

#[test]
fn cosine_similarity_cases() {
    let v = vec![1.0, 2.0, 3.0];
    let sim_same = cosine_similarity(&v, &v, auto()).unwrap();
    assert!((sim_same - 1.0).abs() < 1e-6);

    let sim_orth = cosine_similarity(&[1.0, 0.0], &[0.0, 1.0], auto()).unwrap();
    assert!(sim_orth.abs() < 1e-6);
}

// A metric spells its name the same way through serde, as_str and FromStr.
#[test]
fn a_metric_spells_the_same_everywhere() {
    use piramid_hardware::compute::Metric;

    for metric in [Metric::Cosine, Metric::Euclidean, Metric::DotProduct] {
        let name = metric.as_str();
        assert_eq!(
            serde_json::to_string(&metric).unwrap(),
            format!("\"{name}\""),
            "serde and as_str disagree for {metric:?}"
        );
        assert_eq!(name.parse::<Metric>().unwrap(), metric, "FromStr disagrees");
    }
}

#[test]
fn an_unknown_metric_names_the_alternatives() {
    use piramid_hardware::compute::Metric;

    let error = "dot_product".parse::<Metric>().unwrap_err().to_string();
    assert!(error.contains("cosine"), "{error}");
    assert!(error.contains("dot"), "{error}");
}

/// Deterministic filler in the range -1 to 1.
fn filler(count: usize, seed: u64) -> Vec<f32> {
    let mut state = seed;
    (0..count)
        .map(|_| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            f32::from(((state >> 33) & 0xFFFF) as u16) / 32_768.0 - 1.0
        })
        .collect()
}

/// Every available strategy scores a batch within 1e-5 of the scalar reference, row for row.
#[test]
fn every_batch_kernel_matches_the_scalar_reference() {
    use piramid_hardware::compute::strategies::{all, ScalarStrategy};

    let reference = ScalarStrategy;
    for dim in [1, 7, 8, 13, 384, 1536] {
        let rows = 300;
        let query = filler(dim, 7 + dim as u64);
        let mut candidates = filler(rows * dim, 11 + dim as u64);
        candidates[..dim].fill(0.0);

        for kernels in all().into_iter().filter(|k| k.is_available()) {
            type Batch = fn(
                &dyn DistanceKernels,
                &[f32],
                &[f32],
                usize,
                &mut [f32],
            ) -> piramid_hardware::compute::ComputeResult<()>;
            type Pair = fn(&ScalarStrategy, &[f32], &[f32]) -> f32;
            let cases: [(&str, Batch, Pair, bool); 3] = [
                (
                    "cosine",
                    |k, q, c, d, o| k.cosine_batch(q, c, d, o),
                    |s, a, b| s.cosine(a, b).unwrap(),
                    false,
                ),
                (
                    "dot",
                    |k, q, c, d, o| k.dot_batch(q, c, d, o),
                    |s, a, b| s.dot(a, b).unwrap(),
                    true,
                ),
                (
                    "euclidean",
                    |k, q, c, d, o| k.euclidean_batch(q, c, d, o),
                    |s, a, b| s.euclidean(a, b).unwrap(),
                    true,
                ),
            ];
            for (name, batch, pair, unbounded) in cases {
                let mut out = vec![f32::NAN; rows];
                batch(kernels, &query, &candidates, dim, &mut out).unwrap();
                for (i, (row, got)) in candidates.chunks_exact(dim).zip(&out).enumerate() {
                    let want = pair(&reference, &query, row);
                    // The rounding bound scales with the magnitude of the summed terms.
                    if name == "cosine" && row.iter().all(|x| *x == 0.0) {
                        assert!(
                            got.is_nan(),
                            "{} cosine dim {dim} row {i}: {got}",
                            kernels.name()
                        );
                        continue;
                    }
                    let magnitude = reference.dot(&query, &query).unwrap().sqrt()
                        * reference.dot(row, row).unwrap().sqrt()
                        + reference.euclidean_squared(&query, row).unwrap();
                    let tolerance = 1e-5 * if unbounded { magnitude.max(1.0) } else { 1.0 };
                    assert!(
                        (got - want).abs() <= tolerance,
                        "{} {name} dim {dim} row {i}: {got} != {want}",
                        kernels.name()
                    );
                }
            }
        }
    }
}

/// A batch kernel refuses a slab that does not match its query and output, on every strategy.
#[test]
fn every_batch_kernel_rejects_a_misshapen_slab() {
    use piramid_hardware::compute::strategies::all;

    for kernels in all().into_iter().filter(|k| k.is_available()) {
        let mut out = [0.0; 2];
        assert!(kernels
            .cosine_batch(&[1.0; 3], &[1.0; 5], 3, &mut out)
            .is_err());
        assert!(kernels
            .dot_batch(&[1.0; 3], &[1.0; 9], 3, &mut out)
            .is_err());
        assert!(kernels
            .euclidean_batch(&[1.0; 2], &[1.0; 6], 3, &mut out)
            .is_err());
    }
}

/// Asking for the GPU is an error in a build without a GPU backend.
#[cfg(not(feature = "gpu-cuda"))]
#[test]
fn the_gpu_mode_is_refused_rather_than_served_by_the_cpu() {
    assert!(for_mode(ExecutionMode::Gpu).is_err());
}

/// A batch row scores identically to the pairwise call.
#[test]
fn a_batch_row_scores_exactly_as_the_pairwise_call_would() {
    let kernels = for_mode(ExecutionMode::Scalar).unwrap();
    let query = [1.0, 2.0, 3.0];
    let rows: [[f32; 3]; 4] = [
        [1.0, 2.0, 3.0],
        [-1.0, -2.0, -3.0],
        [3.0, 2.0, 1.0],
        [0.0, 0.0, 1.0],
    ];
    let slab: Vec<f32> = rows.iter().flatten().copied().collect();

    for metric in [Metric::Cosine, Metric::Euclidean, Metric::DotProduct] {
        let mut batch = vec![0.0; rows.len()];
        metric
            .calculate_batch(&query, &slab, 3, &mut batch, kernels)
            .unwrap();

        for (row, scored) in rows.iter().zip(&batch) {
            let pairwise = metric.calculate(&query, row, kernels).unwrap();
            assert!(
                (pairwise - scored).abs() < f32::EPSILON,
                "{metric:?}: batch {scored} != pairwise {pairwise}"
            );
        }
    }
}

#[test]
fn a_slab_that_is_not_a_whole_number_of_rows_is_refused() {
    let kernels = for_mode(ExecutionMode::Scalar).unwrap();
    let mut out = [0.0; 2];

    // Five floats at width three is not a whole number of rows.
    let error = Metric::Cosine
        .calculate_batch(&[1.0, 2.0, 3.0], &[1.0; 5], 3, &mut out, kernels)
        .unwrap_err();

    assert!(matches!(error, ComputeError::ShapeMismatch { .. }));
}
