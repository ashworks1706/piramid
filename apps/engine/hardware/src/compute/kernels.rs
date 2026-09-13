//! The kernel contract every execution strategy implements; batch methods take a row-major slab.

use crate::compute::error::{ComputeError, ComputeResult};
use crate::compute::mode::ExecutionMode;

/// Distance and similarity kernels for one execution strategy.
pub trait DistanceKernels: Send + Sync {
    /// The execution mode this strategy serves.
    fn mode(&self) -> ExecutionMode;

    /// Stable name for logs, metrics, and error messages.
    fn name(&self) -> &'static str;

    /// Whether this strategy can run on this machine.
    fn is_available(&self) -> bool;

    /// Cosine similarity of two equal-length vectors.
    fn cosine(&self, a: &[f32], b: &[f32]) -> f32;

    /// Inner product of two equal-length vectors.
    fn dot(&self, a: &[f32], b: &[f32]) -> f32;

    /// L2 distance between two equal-length vectors.
    fn euclidean(&self, a: &[f32], b: &[f32]) -> f32;

    /// Squared L2 distance, skipping the final sqrt.
    fn euclidean_squared(&self, a: &[f32], b: &[f32]) -> f32;

    /// Score query against every row of the row-major candidates slab.
    fn cosine_batch(
        &self,
        query: &[f32],
        candidates: &[f32],
        dim: usize,
        out: &mut [f32],
    ) -> ComputeResult<()>;

    /// Inner product of query against every row of the candidates slab.
    fn dot_batch(
        &self,
        query: &[f32],
        candidates: &[f32],
        dim: usize,
        out: &mut [f32],
    ) -> ComputeResult<()>;

    /// L2 distance from query to every row of the candidates slab.
    fn euclidean_batch(
        &self,
        query: &[f32],
        candidates: &[f32],
        dim: usize,
        out: &mut [f32],
    ) -> ComputeResult<()>;
}

/// Validate the slab and out shape shared by every batch kernel; returns the row count.
pub fn check_batch_shape(
    query: &[f32],
    candidates: &[f32],
    dim: usize,
    out: &[f32],
) -> ComputeResult<usize> {
    if dim == 0 {
        return Err(ComputeError::ShapeMismatch {
            expected: 1,
            got: 0,
        });
    }
    if query.len() != dim {
        return Err(ComputeError::ShapeMismatch {
            expected: dim,
            got: query.len(),
        });
    }
    if !candidates.len().is_multiple_of(dim) {
        return Err(ComputeError::ShapeMismatch {
            expected: candidates.len().next_multiple_of(dim),
            got: candidates.len(),
        });
    }
    let rows = candidates.len() / dim;
    if out.len() != rows {
        return Err(ComputeError::ShapeMismatch {
            expected: rows,
            got: out.len(),
        });
    }
    Ok(rows)
}
