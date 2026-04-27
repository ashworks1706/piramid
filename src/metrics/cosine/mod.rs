// Cosine similarity between two vectors
// Returns value in range [-1, 1] where:
// 1.0 = identical direction
// 0.0 = orthogonal (perpendicular)
// -1.0 = opposite direction

mod scalar;
mod simd;
mod parallel;
mod binary;
mod jit;

use crate::config::ExecutionMode;
pub use scalar::cosine_similarity_scalar;
pub use simd::cosine_similarity_simd;
pub use parallel::cosine_similarity_parallel;
pub use binary::cosine_similarity_binary;
pub use jit::cosine_similarity_jit;

pub fn cosine_similarity(a: &[f32], b: &[f32], mode: ExecutionMode) -> f32 {
    let resolved = mode.resolve();
    match resolved {
        ExecutionMode::Simd => cosine_similarity_simd(a, b),
        ExecutionMode::Scalar => cosine_similarity_scalar(a, b),
        ExecutionMode::Parallel => cosine_similarity_parallel(a, b),
        ExecutionMode::Binary => cosine_similarity_binary(a, b),
        ExecutionMode::Jit => cosine_similarity_jit(a, b),
        _ => cosine_similarity_scalar(a, b),
    }
}
