//! Distance and similarity math, and the strategy dispatch that runs it.

pub mod error;
pub mod kernels;
pub mod metric;
pub mod mode;
pub mod pairwise;
pub mod quantization;
pub mod strategies;

pub use error::{ComputeError, ComputeResult};
pub use kernels::{check_batch_shape, check_pair_shape, DistanceKernels};
pub use metric::Metric;
pub use mode::ExecutionMode;
pub use pairwise::{
    cosine_similarity, dot_product, euclidean_distance, euclidean_distance_squared,
};
