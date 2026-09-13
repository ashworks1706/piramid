//! Rewrites the record store without dead entries, resuming any compaction a crash interrupted.

use std::collections::HashMap;

use super::Collection;
use crate::storage::record_store::RecordStore;
use crate::storage::sidecars::{
    remove_if_present, replace_file, sync_parent_dir, tmp_path, write_atomic, EntryPointer,
};
use crate::storage::SidecarManager;
use piramid_core::error::Result;

/// Compacts a collection by rewriting its live documents into a fresh record file.
pub fn compact(collection: &mut Collection) -> Result<CompactStats> {
    collection.ensure_writable()?;
    let base = collection.path.clone();
    let sidecars = SidecarManager::at(&base);
    recover(&base)?;

    let bytes_before = collection.record_store.used_bytes();
    super::checkpoint::checkpoint(collection)?;

    let compact_path = sidecars.compact_path();
    let mut compacted = RecordStore::open(&compact_path, &collection.config, &HashMap::new())?;
    let mut offsets = HashMap::with_capacity(collection.offsets.len());
    for (id, pointer) in &collection.offsets {
        let document = collection.record_store.read_document(pointer)?;
        let moved = compacted.append(&RecordStore::encode_document(&document)?)?;
        offsets.insert(*id, moved);
    }
    compacted.sync()?;
    drop(compacted);
    sync_parent_dir(&compact_path)?;
    sidecars.save_compact_offsets(&offsets)?;

    let adopted = write_atomic(&sidecars.compact_commit_path(), &[])
        .and_then(|()| adopt_committed(collection, offsets));
    if let Err(error) = adopted {
        tracing::error!(
            target: "piramid::writes",
            collection = %collection.manifest.name,
            %error,
            "a committed compaction could not be finished; writes are refused until reopen"
        );
        collection.unfinished_compaction = Some(error.to_string());
        return Err(error);
    }
    collection
        .manifest
        .update_vector_count(collection.offsets.len())?;

    Ok(CompactStats {
        documents: collection.offsets.len(),
        bytes_before,
        bytes_after: collection.record_store.used_bytes(),
    })
}

/// Finishes the committed compaction on disk and points the collection at the compacted files.
fn adopt_committed(
    collection: &mut Collection,
    offsets: HashMap<uuid::Uuid, EntryPointer>,
) -> Result<()> {
    recover(&collection.path)?;
    collection.record_store = RecordStore::open(&collection.path, &collection.config, &offsets)?;
    collection.offsets = offsets;
    Ok(())
}

/// Finishes a committed compaction of the collection at base, or discards an uncommitted one.
pub(crate) fn recover(base: &str) -> Result<()> {
    let sidecars = SidecarManager::at(base);
    if std::fs::exists(sidecars.compact_commit_path())? {
        tracing::info!(
            target: "piramid::writes",
            collection = base,
            "finishing an interrupted compaction"
        );
        finish_committed(&sidecars, base)
    } else {
        discard_uncommitted(&sidecars)
    }
}

/// Moves the compacted files that are still present into place, then removes the commit marker.
fn finish_committed(sidecars: &SidecarManager<'_>, base: &str) -> Result<()> {
    let compact_path = sidecars.compact_path();
    if std::fs::exists(&compact_path)? {
        replace_file(&compact_path, base)?;
    }
    let compact_offsets_path = sidecars.compact_offsets_path();
    if std::fs::exists(&compact_offsets_path)? {
        replace_file(&compact_offsets_path, &sidecars.offsets_path())?;
    }
    let commit_path = sidecars.compact_commit_path();
    remove_if_present(&commit_path)?;
    sync_parent_dir(&commit_path)
}

/// Delete every file an uncommitted compaction may have left.
fn discard_uncommitted(sidecars: &SidecarManager<'_>) -> Result<()> {
    let compact_offsets_path = sidecars.compact_offsets_path();
    for path in [
        sidecars.compact_path(),
        tmp_path(&compact_offsets_path),
        compact_offsets_path,
        tmp_path(&sidecars.compact_commit_path()),
    ] {
        remove_if_present(&path)?;
    }
    Ok(())
}

/// What a compaction kept and reclaimed.
#[derive(Debug)]
pub struct CompactStats {
    /// Live documents rewritten into the new record file.
    pub documents: usize,
    /// Bytes of records in the data file before compaction, dead entries included.
    pub bytes_before: u64,
    /// Bytes of records in the data file after compaction.
    pub bytes_after: u64,
}
