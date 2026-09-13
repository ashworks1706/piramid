//! Sidecar and mmap helpers. [SidecarManager] is the entry point.

mod durable;
mod manager;
mod mmap;
mod offsets;
mod warm;

pub(crate) use durable::{
    remove_if_present, replace_file, sync_parent_dir, tmp_path, write_atomic,
};
pub use manager::SidecarManager;
pub(crate) use mmap::{create_mmap, ensure_file_size, grow_mmap_if_needed, mapped_or_file_len};
pub use offsets::EntryPointer;
pub use warm::warm_file;
pub(crate) use warm::warm_mmap;
