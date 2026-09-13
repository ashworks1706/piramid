//! Similarity metrics: a [Metric] is what to measure, independent of the strategy measuring it.

use serde::{Deserialize, Serialize};

use crate::compute::error::{ComputeError, ComputeResult};
use crate::compute::kernels::DistanceKernels;
use crate::compute::pairwise::{cosine_similarity, dot_product, euclidean_distance};

/// How similarity between two vectors is measured. [Metric::calculate] gives closer vectors a
/// higher score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Metric {
    /// Angle between vectors, in the range -1 to 1.
    #[default]
    Cosine,
    /// L2 distance, mapped to 1 / (1 + d).
    Euclidean,
    /// Unnormalized inner product.
    #[serde(rename = "dot")]
    DotProduct,
}

impl Metric {
    /// Score a against b; a higher result means more similar.
    pub fn calculate(
        &self,
        a: &[f32],
        b: &[f32],
        kernels: &dyn DistanceKernels,
    ) -> ComputeResult<f32> {
        Ok(match self {
            Metric::Cosine => cosine_similarity(a, b, kernels)?,
            Metric::Euclidean => 1.0 / (1.0 + euclidean_distance(a, b, kernels)?),
            Metric::DotProduct => dot_product(a, b, kernels)?,
        })
    }

    /// Score query against every row of the row-major candidates slab, into out.
    ///
    /// Row i of out is exactly [Metric::calculate] against row i of the slab, including the
    /// Euclidean higher-is-closer transform.
    pub fn calculate_batch(
        &self,
        query: &[f32],
        candidates: &[f32],
        dim: usize,
        out: &mut [f32],
        kernels: &dyn DistanceKernels,
    ) -> ComputeResult<()> {
        match self {
            Metric::Cosine => kernels.cosine_batch(query, candidates, dim, out),
            Metric::DotProduct => kernels.dot_batch(query, candidates, dim, out),
            Metric::Euclidean => {
                kernels.euclidean_batch(query, candidates, dim, out)?;
                for slot in out.iter_mut() {
                    *slot = 1.0 / (1.0 + *slot);
                }
                Ok(())
            }
        }
    }

    /// Stable lowercase name. Matches the serde representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Metric::Cosine => "cosine",
            Metric::Euclidean => "euclidean",
            Metric::DotProduct => "dot",
        }
    }
}

impl std::str::FromStr for Metric {
    type Err = ComputeError;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        [Metric::Cosine, Metric::Euclidean, Metric::DotProduct]
            .into_iter()
            .find(|metric| metric.as_str() == name)
            .ok_or_else(|| ComputeError::UnknownMetric {
                name: name.to_string(),
            })
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "a failed assertion is the point of a test"
)]
mod tests {
    use super::*;
    use crate::compute::strategies::for_mode;
    use crate::compute::ExecutionMode;

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

        assert!(matches!(
            error,
            crate::compute::ComputeError::ShapeMismatch { .. }
        ));
    }
}
