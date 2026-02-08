// Storage module - handles vector persistence with memory-mapped files

mod entry;
mod collection;
mod utils;
pub mod wal;

pub use entry::Document;
pub use collection::Collection;
