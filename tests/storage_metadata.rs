use piramid::storage::CollectionMetadata;
use std::thread;
use std::time::Duration;

#[test]
fn metadata_new_and_dimensions() {
    let mut meta = CollectionMetadata::new("test".into());
    assert_eq!(meta.name, "test");
    assert_eq!(meta.dimensions, None);
    meta.set_dimensions(512);
    assert_eq!(meta.dimensions, Some(512));
    meta.set_dimensions(1024);
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
    let path = ".piramid/tests/corrupt_pointer_index.db";
    std::fs::create_dir_all(".piramid/tests").unwrap();
    let index_path = format!("{path}.index.db");
    let _ = std::fs::remove_file(&index_path);
    std::fs::write(&index_path, b"not bincode").unwrap();

    let result = piramid::storage::persistence::load_index(path);
    assert!(result.is_err());

    let _ = std::fs::remove_file(&index_path);
}
