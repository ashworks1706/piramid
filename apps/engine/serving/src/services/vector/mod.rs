//! Document insert, read, delete and search operations.

mod read;
mod search;
mod write;

pub use read::{get_vector, list_vectors};
pub use search::search_vectors;
pub use write::{delete_vector, delete_vectors, insert_vector, upsert_vector};

pub(super) const MAX_BATCH_SIZE: usize = 10_000;
