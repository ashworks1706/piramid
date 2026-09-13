//! Similarity metrics: a [Metric] is what to measure, independent of the strategy measuring it.

use serde::{Deserialize, Serialize};

use crate::compute::error::{ComputeError, ComputeResult};
use crate::compute::kernels::DistanceKernels;
use crate::compute::pairwise::{cosine_similarity, dot_product, euclidean_distance};

/// How similarity between two vectors is measured; [Metric::calculate] scores closer as higher.
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

    /// Scores query against every row of the row-major candidates slab, into out.
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
