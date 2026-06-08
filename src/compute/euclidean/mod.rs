// Euclidean distance between two vectors
// Measures the straight-line distance in vector space

mod binary;
mod jit;
mod parallel;
mod scalar;
mod simd;

use crate::config::ExecutionMode;
pub use binary::{euclidean_distance_binary, euclidean_distance_squared_binary};
pub use jit::{euclidean_distance_jit, euclidean_distance_squared_jit};
pub use parallel::{euclidean_distance_parallel, euclidean_distance_squared_parallel};
pub use scalar::{euclidean_distance_scalar, euclidean_distance_squared_scalar};
pub use simd::{euclidean_distance_simd, euclidean_distance_squared_simd};

pub fn euclidean_distance(a: &[f32], b: &[f32], mode: ExecutionMode) -> f32 {
    let resolved = mode.resolve();
    match resolved {
        ExecutionMode::Simd => euclidean_distance_simd(a, b),
        ExecutionMode::Scalar => euclidean_distance_scalar(a, b),
        ExecutionMode::Parallel => euclidean_distance_parallel(a, b),
        ExecutionMode::Binary => euclidean_distance_binary(a, b),
        ExecutionMode::Jit => euclidean_distance_jit(a, b),
        ExecutionMode::Gpu => panic!("GPU euclidean distance is not implemented"),
        ExecutionMode::Auto => unreachable!("ExecutionMode::Auto should resolve before dispatch"),
    }
}

pub fn euclidean_distance_squared(a: &[f32], b: &[f32], mode: ExecutionMode) -> f32 {
    let resolved = mode.resolve();
    match resolved {
        ExecutionMode::Simd => euclidean_distance_squared_simd(a, b),
        ExecutionMode::Scalar => euclidean_distance_squared_scalar(a, b),
        ExecutionMode::Parallel => euclidean_distance_squared_parallel(a, b),
        ExecutionMode::Binary => euclidean_distance_squared_binary(a, b),
        ExecutionMode::Jit => euclidean_distance_squared_jit(a, b),
        ExecutionMode::Gpu => panic!("GPU euclidean squared distance is not implemented"),
        ExecutionMode::Auto => unreachable!("ExecutionMode::Auto should resolve before dispatch"),
    }
}
