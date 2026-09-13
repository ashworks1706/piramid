//! Explicitly vectorized CPU strategy using the f32x8 type from wide, compiled to the instruction
//! set the build targets.

use wide::f32x8;

use crate::compute::error::ComputeResult;
use crate::compute::kernels::{check_batch_shape, check_pair_shape, DistanceKernels};
use crate::compute::mode::ExecutionMode;

/// SIMD CPU kernels.
#[derive(Debug, Default, Clone, Copy)]
pub struct SimdStrategy;

/// Load an 8-element chunk into a lane vector. Panics on a chunk shorter than 8.
#[inline(always)]
fn load(chunk: &[f32]) -> f32x8 {
    f32x8::new([
        chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
    ])
}

/// Sum the lanes of an accumulator.
#[inline(always)]
fn total(lanes: f32x8) -> f32 {
    lanes.to_array().iter().sum()
}

/// Sum of squares of a vector.
#[inline(always)]
pub(super) fn norm_squared(a: &[f32]) -> f32 {
    let mut sum = f32x8::splat(0.0);
    let chunks = a.chunks_exact(8);
    let rem = chunks.remainder();
    for chunk in chunks {
        let v = load(chunk);
        sum += v * v;
    }
    let mut result = total(sum);
    for &x in rem {
        result += x * x;
    }
    result
}

/// Inner product of a and b, and the sum of squares of b, in one pass.
#[inline(always)]
pub(super) fn dot_and_norm_squared(a: &[f32], b: &[f32]) -> (f32, f32) {
    let mut dot_sum = f32x8::splat(0.0);
    let mut norm_sum = f32x8::splat(0.0);
    let a_chunks = a.chunks_exact(8);
    let b_chunks = b.chunks_exact(8);
    let a_rem = a_chunks.remainder();
    let b_rem = b_chunks.remainder();
    for (ca, cb) in a_chunks.zip(b_chunks) {
        let va = load(ca);
        let vb = load(cb);
        dot_sum += va * vb;
        norm_sum += vb * vb;
    }
    let mut dot = total(dot_sum);
    let mut norm_b = total(norm_sum);
    for (&x, &y) in a_rem.iter().zip(b_rem) {
        dot += x * y;
        norm_b += y * y;
    }
    (dot, norm_b)
}

/// Inner product of a and b.
#[inline(always)]
pub(super) fn dot(a: &[f32], b: &[f32]) -> f32 {
    let mut sum = f32x8::splat(0.0);
    let a_chunks = a.chunks_exact(8);
    let b_chunks = b.chunks_exact(8);
    let a_rem = a_chunks.remainder();
    let b_rem = b_chunks.remainder();
    for (ca, cb) in a_chunks.zip(b_chunks) {
        sum += load(ca) * load(cb);
    }
    let mut result = total(sum);
    for (&x, &y) in a_rem.iter().zip(b_rem) {
        result += x * y;
    }
    result
}

/// Squared L2 distance between a and b.
#[inline(always)]
pub(super) fn euclidean_squared(a: &[f32], b: &[f32]) -> f32 {
    let mut sum = f32x8::splat(0.0);
    let a_chunks = a.chunks_exact(8);
    let b_chunks = b.chunks_exact(8);
    let a_rem = a_chunks.remainder();
    let b_rem = b_chunks.remainder();
    for (ca, cb) in a_chunks.zip(b_chunks) {
        let diff = load(ca) - load(cb);
        sum += diff * diff;
    }
    let mut result = total(sum);
    for (&x, &y) in a_rem.iter().zip(b_rem) {
        let diff = x - y;
        result += diff * diff;
    }
    result
}

/// Cosine from an inner product and the two sums of squares; NaN when either norm is zero.
#[inline(always)]
pub(super) fn cosine_from_parts(dot: f32, norm_a: f32, norm_b: f32) -> f32 {
    let denominator = norm_a.sqrt() * norm_b.sqrt();
    if denominator == 0.0 {
        f32::NAN
    } else {
        dot / denominator
    }
}

impl DistanceKernels for SimdStrategy {
    fn mode(&self) -> ExecutionMode {
        ExecutionMode::Simd
    }

    fn name(&self) -> &'static str {
        "simd"
    }

    fn is_available(&self) -> bool {
        cfg!(any(target_arch = "x86_64", target_arch = "aarch64"))
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
