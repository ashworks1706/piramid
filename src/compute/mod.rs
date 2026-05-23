pub mod cosine;
pub mod dot;
pub mod euclidean;

pub use cosine::cosine_similarity;
pub use dot::dot_product;
pub use euclidean::{euclidean_distance, euclidean_distance_squared};
