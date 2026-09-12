//! Write-ahead log: its entries and the file they are appended to.

mod entry;
mod log;

pub use entry::WalEntry;
pub use log::Wal;
