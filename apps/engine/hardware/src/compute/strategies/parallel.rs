//! Rayon-parallel CPU strategy: pairwise operands chunked across cores, batch rows fanned across
//! cores with each row scored by the SIMD kernels.

use rayon::prelude::*;

use crate::compute::error::ComputeResult;
use crate::compute::kernels::{check_batch_shape, DistanceKernels};
use crate::compute::mode::ExecutionMode;
use crate::compute::strategies::simd;

/// Multi-threaded CPU kernels.
#[derive(Debug, Default, Clone, Copy)]
pub struct ParallelStrategy;

/// Chunk width used to split an operand of the given length across threads.
fn chunk_size(len: usize) -> usize {
    (len / num_cpus::get()).max(1024)
}

/// Floats a batch task scores at minimum, so a thread is not handed less work than its dispatch.
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
        let width = chunk_size(a.len());

        let (dot, norm_a, norm_b): (f32, f32, f32) = a
            .par_chunks(width)
            .zip(b.par_chunks(width))
            .map(|(chunk_a, chunk_b)| {
                let mut dot = 0.0;
                let mut norm_a = 0.0;
                let mut norm_b = 0.0;
                for (&x, &y) in chunk_a.iter().zip(chunk_b) {
                    dot += x * y;
                    norm_a += x * x;
                    norm_b += y * y;
                }
                (dot, norm_a, norm_b)
            })
            .reduce(
                || (0.0, 0.0, 0.0),
                |(d1, na1, nb1), (d2, na2, nb2)| (d1 + d2, na1 + na2, nb1 + nb2),
            );

        let denominator = norm_a.sqrt() * norm_b.sqrt();
        if denominator == 0.0 {
            0.0
        } else {
            dot / denominator
        }
    }

    fn dot(&self, a: &[f32], b: &[f32]) -> f32 {
        let width = chunk_size(a.len());
        a.par_chunks(width)
            .zip(b.par_chunks(width))
            .map(|(chunk_a, chunk_b)| {
                let mut sum = 0.0;
                for (&x, &y) in chunk_a.iter().zip(chunk_b) {
                    sum += x * y;
                }
                sum
            })
            .sum()
    }

    fn euclidean(&self, a: &[f32], b: &[f32]) -> f32 {
        self.euclidean_squared(a, b).sqrt()
    }

    fn euclidean_squared(&self, a: &[f32], b: &[f32]) -> f32 {
        let width = chunk_size(a.len());
        a.par_chunks(width)
            .zip(b.par_chunks(width))
            .map(|(chunk_a, chunk_b)| {
                let mut sum = 0.0;
                for (&x, &y) in chunk_a.iter().zip(chunk_b) {
                    let diff = x - y;
                    sum += diff * diff;
                }
                sum
            })
            .sum()
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
