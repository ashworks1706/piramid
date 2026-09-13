//! Portable scalar strategy: no intrinsics or threads, the correctness reference for the others.

use crate::compute::error::ComputeResult;
use crate::compute::kernels::{check_batch_shape, check_pair_shape, DistanceKernels};
use crate::compute::mode::ExecutionMode;

/// Scalar CPU kernels.
#[derive(Debug, Default, Clone, Copy)]
pub struct ScalarStrategy;

/// Sum of squares of a vector.
pub(super) fn norm_squared(a: &[f32]) -> f32 {
    let mut sum = 0.0;
    for &x in a {
        sum += x * x;
    }
    sum
}

/// Inner product of a and b, and the sum of squares of b, in one pass.
pub(super) fn dot_and_norm_squared(a: &[f32], b: &[f32]) -> (f32, f32) {
    let mut dot = 0.0;
    let mut norm_b = 0.0;
    for (&x, &y) in a.iter().zip(b) {
        dot += x * y;
        norm_b += y * y;
    }
    (dot, norm_b)
}

/// Inner product of a and b.
pub(super) fn dot(a: &[f32], b: &[f32]) -> f32 {
    let mut result = 0.0;
    for (&x, &y) in a.iter().zip(b) {
        result += x * y;
    }
    result
}

/// Squared L2 distance between a and b.
pub(super) fn euclidean_squared(a: &[f32], b: &[f32]) -> f32 {
    let mut sum_sq = 0.0;
    for (&x, &y) in a.iter().zip(b) {
        let diff = x - y;
        sum_sq += diff * diff;
    }
    sum_sq
}

/// Cosine from an inner product and the two sums of squares; NaN when either norm is zero.
#[inline]
pub(super) fn cosine_from_parts(dot: f32, norm_a: f32, norm_b: f32) -> f32 {
    let denominator = norm_a.sqrt() * norm_b.sqrt();
    if denominator == 0.0 {
        f32::NAN
    } else {
        dot / denominator
    }
}

impl DistanceKernels for ScalarStrategy {
    fn mode(&self) -> ExecutionMode {
        ExecutionMode::Scalar
    }

    fn name(&self) -> &'static str {
        "scalar"
    }

    fn is_available(&self) -> bool {
        true
    }

    fn cosine(&self, a: &[f32], b: &[f32]) -> ComputeResult<f32> {
        check_pair_shape(a, b)?;
        let (dot, norm_b) = dot_and_norm_squared(a, b);
        Ok(cosine_from_parts(dot, norm_squared(a), norm_b))
    }

    fn dot(&self, a: &[f32], b: &[f32]) -> ComputeResult<f32> {
        check_pair_shape(a, b)?;
        Ok(dot(a, b))
    }

    fn euclidean(&self, a: &[f32], b: &[f32]) -> ComputeResult<f32> {
        check_pair_shape(a, b)?;
        Ok(euclidean_squared(a, b).sqrt())
    }

    fn euclidean_squared(&self, a: &[f32], b: &[f32]) -> ComputeResult<f32> {
        check_pair_shape(a, b)?;
        Ok(euclidean_squared(a, b))
    }

    fn cosine_batch(
        &self,
        query: &[f32],
        candidates: &[f32],
        dim: usize,
        out: &mut [f32],
    ) -> ComputeResult<()> {
        check_batch_shape(query, candidates, dim, out)?;
        let norm_query = norm_squared(query);
        for (row, slot) in candidates.chunks_exact(dim).zip(out.iter_mut()) {
            let (dot, norm_row) = dot_and_norm_squared(query, row);
            *slot = cosine_from_parts(dot, norm_query, norm_row);
        }
        Ok(())
    }

    fn dot_batch(
        &self,
        query: &[f32],
        candidates: &[f32],
        dim: usize,
        out: &mut [f32],
    ) -> ComputeResult<()> {
        check_batch_shape(query, candidates, dim, out)?;
        for (row, slot) in candidates.chunks_exact(dim).zip(out.iter_mut()) {
            *slot = dot(query, row);
        }
        Ok(())
    }

    fn euclidean_batch(
        &self,
        query: &[f32],
        candidates: &[f32],
        dim: usize,
        out: &mut [f32],
    ) -> ComputeResult<()> {
        check_batch_shape(query, candidates, dim, out)?;
        for (row, slot) in candidates.chunks_exact(dim).zip(out.iter_mut()) {
            *slot = euclidean_squared(query, row).sqrt();
        }
        Ok(())
    }
}
