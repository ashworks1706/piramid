// Dot product of two vectors
// Fast similarity metric for normalized vectors

mod binary;
mod jit;
mod parallel;
mod scalar;
mod simd;

use crate::config::ExecutionMode;
pub use binary::dot_product_binary;
pub use jit::dot_product_jit;
pub use parallel::dot_product_parallel;
pub use scalar::dot_product_scalar;
pub use simd::dot_product_simd;

pub fn dot_product(a: &[f32], b: &[f32], mode: ExecutionMode) -> f32 {
    let resolved = mode.resolve();
    match resolved {
        ExecutionMode::Simd => dot_product_simd(a, b),
        ExecutionMode::Scalar => dot_product_scalar(a, b),
        ExecutionMode::Parallel => dot_product_parallel(a, b),
        ExecutionMode::Binary => dot_product_binary(a, b),
        ExecutionMode::Jit => dot_product_jit(a, b),
        ExecutionMode::Gpu => panic!("GPU dot product is not implemented"),
        ExecutionMode::Auto => unreachable!("ExecutionMode::Auto should resolve before dispatch"),
    }
}
