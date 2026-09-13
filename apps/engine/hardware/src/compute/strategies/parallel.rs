//! Rayon-parallel CPU strategy: batch rows fanned across cores and scored by the SIMD kernels. A
//! single pair runs the SIMD kernel on the calling thread.

use rayon::prelude::*;

use crate::compute::error::ComputeResult;
use crate::compute::kernels::{check_batch_shape, DistanceKernels};
use crate::compute::mode::ExecutionMode;
use crate::compute::strategies::simd;

/// Multi-threaded CPU kernels.
#[derive(Debug, Default, Clone, Copy)]
pub struct ParallelStrategy;

/// Fewest floats a batch task scores.
const MIN_TASK_FLOATS: usize = 1 << 15;

/// Fewest rows a batch task takes at the given row width.
fn min_task_rows(dim: usize) -> usize {
    (MIN_TASK_FLOATS / dim).max(1)
}

impl DistanceKernels for ParallelStrategy {
    fn mode(&self) -> ExecutionMode {
        ExecutionMode::Parallel
    }

    fn name(&self) -> &'static str {
        "parallel"
    }

    fn is_available(&self) -> bool {
        cfg!(any(target_arch = "x86_64", target_arch = "aarch64"))
    }

    fn cosine(&self, a: &[f32], b: &[f32]) -> f32 {
        let (dot, norm_b) = simd::dot_and_norm_squared(a, b);
        simd::cosine_from_parts(dot, simd::norm_squared(a), norm_b)
    }

    fn dot(&self, a: &[f32], b: &[f32]) -> f32 {
        simd::dot(a, b)
    }

    fn euclidean(&self, a: &[f32], b: &[f32]) -> f32 {
        simd::euclidean_squared(a, b).sqrt()
    }

    fn euclidean_squared(&self, a: &[f32], b: &[f32]) -> f32 {
        simd::euclidean_squared(a, b)
    }

    fn cosine_batch(
        &self,
        query: &[f32],
        candidates: &[f32],
        dim: usize,
        out: &mut [f32],
    ) -> ComputeResult<()> {
        check_batch_shape(query, candidates, dim, out)?;
        let norm_query = simd::norm_squared(query);
        out.par_iter_mut()
            .zip(candidates.par_chunks_exact(dim))
            .with_min_len(min_task_rows(dim))
            .for_each(|(slot, row)| {
                let (dot, norm_row) = simd::dot_and_norm_squared(query, row);
                *slot = simd::cosine_from_parts(dot, norm_query, norm_row);
            });
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
        out.par_iter_mut()
            .zip(candidates.par_chunks_exact(dim))
            .with_min_len(min_task_rows(dim))
            .for_each(|(slot, row)| *slot = simd::dot(query, row));
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
        out.par_iter_mut()
            .zip(candidates.par_chunks_exact(dim))
            .with_min_len(min_task_rows(dim))
            .for_each(|(slot, row)| *slot = simd::euclidean_squared(query, row).sqrt());
        Ok(())
    }
}
