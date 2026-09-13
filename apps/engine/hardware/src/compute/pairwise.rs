//! Single-pair distance entry points over a caller-resolved strategy.

use crate::compute::error::ComputeResult;
use crate::compute::kernels::DistanceKernels;

/// Cosine similarity in the range -1 to 1; NaN if either operand is a zero vector.
pub fn cosine_similarity(
    a: &[f32],
    b: &[f32],
    kernels: &dyn DistanceKernels,
) -> ComputeResult<f32> {
    kernels.cosine(a, b)
}

/// Inner product of two vectors.
pub fn dot_product(a: &[f32], b: &[f32], kernels: &dyn DistanceKernels) -> ComputeResult<f32> {
    kernels.dot(a, b)
}

/// L2 distance between two vectors.
pub fn euclidean_distance(
    a: &[f32],
    b: &[f32],
    kernels: &dyn DistanceKernels,
) -> ComputeResult<f32> {
    kernels.euclidean(a, b)
}

/// Squared L2 distance, skipping the final sqrt. Orders pairs the same as euclidean_distance.
pub fn euclidean_distance_squared(
    a: &[f32],
    b: &[f32],
    kernels: &dyn DistanceKernels,
) -> ComputeResult<f32> {
    kernels.euclidean_squared(a, b)
}
