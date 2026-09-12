#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]
//! The collection manifest and the offset index on disk.

use piramid_database::storage::CollectionMetadata;
use std::thread;
use std::time::Duration;

#[test]
fn metadata_new_and_dimensions() {
    let mut meta = CollectionMetadata::new("test".into());
    assert_eq!(meta.name, "test");
    assert_eq!(meta.dimensions, None);
    meta.set_dimensions(512).unwrap();
    assert_eq!(meta.dimensions, Some(512));
    meta.set_dimensions(512).unwrap();

    assert!(
        meta.set_dimensions(1024).is_err(),
        "a disagreeing width is a corrupt manifest, not a no-op"
    );
    assert_eq!(meta.dimensions, Some(512));
}

#[test]
fn metadata_touch_and_counts() {
    let mut meta = CollectionMetadata::new("test".into());
    let created = meta.created_at;
    let prev_updated = meta.updated_at;
    thread::sleep(Duration::from_millis(2));
    meta.touch();
    assert!(meta.updated_at >= prev_updated);
    assert_eq!(meta.created_at, created);

    thread::sleep(Duration::from_millis(2));
    meta.update_vector_count(100);
    assert_eq!(meta.vector_count, 100);
    assert!(meta.updated_at >= meta.created_at);
}

#[test]
fn corrupt_pointer_index_fails_to_load() {
    let path = concat!(env!("CARGO_TARGET_TMPDIR"), "/corrupt_pointer_index.db");
    std::fs::create_dir_all(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let sidecars = piramid_database::storage::sidecars::SidecarManager::at(path);
    let index_path = sidecars.offsets_path();
    let _ = std::fs::remove_file(&index_path);
    std::fs::write(&index_path, b"not bincode").unwrap();

    assert!(sidecars.load_offsets().is_err());

    let _ = std::fs::remove_file(&index_path);
}
