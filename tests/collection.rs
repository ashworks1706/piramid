use piramid::{
    collections::{compact, CollectionOpenOptions},
    metadata,
    search::SearchParams,
    CacheConfig, Collection, CollectionConfig, Document, MemoryConfig, Metric,
};
use std::fs;

fn ensure_test_dir() {
    let _ = fs::create_dir_all(".piramid/tests");
}

fn cleanup_test_files(paths: &[&str]) {
    ensure_test_dir();
    for path in paths {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn basic_store_and_retrieve() {
    ensure_test_dir();
    let test_path = ".piramid/tests/test_basic.db";
    let files = vec![
        test_path,
        ".piramid/tests/test_basic.db.index.db",
        ".piramid/tests/test_basic.db.wal.db",
        ".piramid/tests/test_basic.db.vecindex.db",
        ".piramid/tests/test_basic.db.metadata.db",
    ];
    cleanup_test_files(&files);

    let mut storage = Collection::open(test_path).unwrap();
    let entry = Document::new(vec![1.0, 2.0, 3.0], "test".to_string());
    let id = storage.insert(entry).unwrap();

    let retrieved = storage.get(&id).unwrap().unwrap();
    assert_eq!(retrieved.text, "test");
    assert_eq!(retrieved.get_vector(), vec![1.0, 2.0, 3.0]);

    drop(storage);
    cleanup_test_files(&files);
}

#[test]
fn persistence_roundtrip() {
    ensure_test_dir();
    let test_path = ".piramid/tests/test_persist.db";
    let files = vec![
        test_path,
        ".piramid/tests/test_persist.db.index.db",
        ".piramid/tests/test_persist.db.wal.db",
        ".piramid/tests/test_persist.db.vecindex.db",
        ".piramid/tests/test_persist.db.metadata.db",
    ];
    cleanup_test_files(&files);

    let id1;
    let id2;
    {
        let mut storage = Collection::open(test_path).unwrap();
        id1 = storage
            .insert(Document::new(vec![1.0, 2.0], "first".into()))
            .unwrap();
        id2 = storage
            .insert(Document::new(vec![3.0, 4.0], "second".into()))
            .unwrap();
    }

    {
        let storage = Collection::open(test_path).unwrap();
        assert_eq!(storage.count(), 2);
        assert_eq!(storage.get(&id1).unwrap().unwrap().text, "first");
        assert_eq!(storage.get(&id2).unwrap().unwrap().text, "second");
    }

    cleanup_test_files(&files);
}

#[test]
fn search_returns_results() {
    ensure_test_dir();
    let test_path = ".piramid/tests/test_search.db";
    let files = vec![
        test_path,
        ".piramid/tests/test_search.db.index.db",
        ".piramid/tests/test_search.db.wal.db",
        ".piramid/tests/test_search.db.vecindex.db",
        ".piramid/tests/test_search.db.metadata.db",
    ];
    cleanup_test_files(&files);

    let mut storage = Collection::open(test_path).unwrap();
    let vectors = [
        vec![1.0, 0.0, 0.0],
        vec![0.0, 1.0, 0.0],
        vec![0.0, 0.0, 1.0],
        vec![0.9, 0.1, 0.0],
    ];
    for (i, vec) in vectors.iter().enumerate() {
        storage
            .insert(Document::new(vec.clone(), format!("vec{}", i)))
            .unwrap();
    }

    let params = SearchParams::default();
    let results = storage
        .search(&[1.0, 0.0, 0.0], 2, Metric::Cosine, params)
        .unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].text, "vec0");

    drop(storage);
    cleanup_test_files(&files);
}

#[test]
fn batch_search_multi_queries() {
    ensure_test_dir();
    let test_path = ".piramid/tests/test_batch_search.db";
    let files = vec![
        test_path,
        ".piramid/tests/test_batch_search.db.index.db",
        ".piramid/tests/test_batch_search.db.wal.db",
        ".piramid/tests/test_batch_search.db.vecindex.db",
        ".piramid/tests/test_batch_search.db.metadata.db",
    ];
    cleanup_test_files(&files);

    let mut storage = Collection::open(test_path).unwrap();
    for i in 0..10 {
        storage
            .insert(Document::new(vec![i as f32, 0.0, 0.0], format!("vec{}", i)))
            .unwrap();
    }

    let queries = vec![
        vec![0.0, 0.0, 0.0],
        vec![5.0, 0.0, 0.0],
        vec![9.0, 0.0, 0.0],
    ];
    let results = storage.search_batch(&queries, 2, Metric::Cosine).unwrap();
    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|hits| !hits.is_empty()));

    drop(storage);
    cleanup_test_files(&files);
}

#[test]
fn no_mmap_insert_grows_file_without_panicking() {
    ensure_test_dir();
    let test_path = ".piramid/tests/test_no_mmap_grow.db";
    let files = vec![
        test_path,
        ".piramid/tests/test_no_mmap_grow.db.index.db",
        ".piramid/tests/test_no_mmap_grow.db.wal.db",
        ".piramid/tests/test_no_mmap_grow.db.wal.meta",
        ".piramid/tests/test_no_mmap_grow.db.vecindex.db",
        ".piramid/tests/test_no_mmap_grow.db.metadata.db",
    ];
    cleanup_test_files(&files);

    let config = CollectionConfig {
        memory: MemoryConfig::no_mmap(),
        ..CollectionConfig::default()
    };

    let mut storage =
        Collection::open_with_options(test_path, CollectionOpenOptions { config }).unwrap();
    let vector = vec![0.25; 1_100_000];
    let id = storage
        .insert(Document::new(
            vector.clone(),
            "large no-mmap document".to_string(),
        ))
        .unwrap();

    let retrieved = storage.get(&id).unwrap().unwrap();
    assert_eq!(retrieved.text, "large no-mmap document");
    assert_eq!(retrieved.get_vector().len(), vector.len());

    drop(storage);
    cleanup_test_files(&files);
}

#[test]
fn updates_write_one_wal_entry_each() {
    ensure_test_dir();
    let test_path = ".piramid/tests/test_update_wal.db";
    let files = vec![
        test_path,
        ".piramid/tests/test_update_wal.db.index.db",
        ".piramid/tests/test_update_wal.db.wal.db",
        ".piramid/tests/test_update_wal.db.vecindex.db",
        ".piramid/tests/test_update_wal.db.metadata.db",
    ];
    cleanup_test_files(&files);

    let mut storage = Collection::open(test_path).unwrap();
    let id = storage
        .insert(Document::with_metadata(
            vec![1.0, 2.0, 3.0],
            "original".to_string(),
            metadata([("kind", "initial".into())]),
        ))
        .unwrap();

    storage
        .update_metadata(&id, metadata([("kind", "updated".into())]))
        .unwrap();
    storage.update_vector(&id, vec![3.0, 2.0, 1.0]).unwrap();

    let wal = fs::read_to_string(format!("{}.wal.db", test_path)).unwrap();
    assert_eq!(
        wal.lines()
            .filter(|line| line.contains("\"Insert\""))
            .count(),
        1
    );
    assert_eq!(
        wal.lines()
            .filter(|line| line.contains("\"Update\""))
            .count(),
        2
    );
    assert_eq!(
        wal.lines()
            .filter(|line| line.contains("\"Delete\""))
            .count(),
        0
    );

    drop(storage);
    cleanup_test_files(&files);
}

#[test]
fn sidecar_files_persist_at_checkpoint_only() {
    ensure_test_dir();
    let test_path = ".piramid/tests/test_checkpoint_only.db";
    let files = vec![
        test_path,
        ".piramid/tests/test_checkpoint_only.db.index.db",
        ".piramid/tests/test_checkpoint_only.db.wal.db",
        ".piramid/tests/test_checkpoint_only.db.vecindex.db",
        ".piramid/tests/test_checkpoint_only.db.metadata.db",
        ".piramid/tests/test_checkpoint_only.db.wal.meta",
    ];
    cleanup_test_files(&files);

    let mut storage = Collection::open(test_path).unwrap();
    storage
        .insert(Document::new(vec![1.0, 2.0, 3.0], "checkpoint only".into()))
        .unwrap();

    assert!(fs::metadata(format!("{}.index.db", test_path)).is_err());
    assert!(fs::metadata(format!("{}.vecindex.db", test_path)).is_err());

    storage.checkpoint().unwrap();

    assert!(fs::metadata(format!("{}.index.db", test_path)).is_ok());
    assert!(fs::metadata(format!("{}.vecindex.db", test_path)).is_ok());

    drop(storage);
    cleanup_test_files(&files);
}

#[test]
fn metadata_cache_is_bounded_without_evicting_vectors() {
    ensure_test_dir();
    let test_path = ".piramid/tests/test_cache_manager_bounds.db";
    let files = vec![
        test_path,
        ".piramid/tests/test_cache_manager_bounds.db.index.db",
        ".piramid/tests/test_cache_manager_bounds.db.wal.db",
        ".piramid/tests/test_cache_manager_bounds.db.vecindex.db",
        ".piramid/tests/test_cache_manager_bounds.db.metadata.db",
        ".piramid/tests/test_cache_manager_bounds.db.wal.meta",
    ];
    cleanup_test_files(&files);

    let config = CollectionConfig {
        cache: CacheConfig::with_size(1),
        ..CollectionConfig::default()
    };
    let mut storage =
        Collection::open_with_options(test_path, CollectionOpenOptions { config }).unwrap();

    let id_a = storage
        .insert(Document::with_metadata(
            vec![1.0, 0.0, 0.0],
            "first".to_string(),
            metadata([("kind", "a".into())]),
        ))
        .unwrap();
    let id_b = storage
        .insert(Document::with_metadata(
            vec![0.0, 1.0, 0.0],
            "second".to_string(),
            metadata([("kind", "b".into())]),
        ))
        .unwrap();

    assert_eq!(storage.get_vectors().len(), 2);
    assert_eq!(storage.metadata_view().len(), 1);
    assert!(storage.get_vectors().contains_key(&id_a));
    assert!(storage.get_vectors().contains_key(&id_b));

    drop(storage);
    cleanup_test_files(&files);
}

#[test]
fn append_cursor_survives_reopen_and_preserves_existing_records() {
    ensure_test_dir();
    let test_path = ".piramid/tests/test_append_cursor_reopen.db";
    let files = vec![
        test_path,
        ".piramid/tests/test_append_cursor_reopen.db.index.db",
        ".piramid/tests/test_append_cursor_reopen.db.wal.db",
        ".piramid/tests/test_append_cursor_reopen.db.vecindex.db",
        ".piramid/tests/test_append_cursor_reopen.db.metadata.db",
        ".piramid/tests/test_append_cursor_reopen.db.wal.meta",
    ];
    cleanup_test_files(&files);

    let first_id = {
        let mut storage = Collection::open(test_path).unwrap();
        let first_id = storage
            .insert(Document::new(vec![1.0, 0.0, 0.0], "first".to_string()))
            .unwrap();
        storage.checkpoint().unwrap();
        first_id
    };

    let second_id = {
        let mut storage = Collection::open(test_path).unwrap();
        storage
            .insert(Document::new(vec![0.0, 1.0, 0.0], "second".to_string()))
            .unwrap()
    };

    let storage = Collection::open(test_path).unwrap();
    assert_eq!(storage.count(), 2);
    assert_eq!(storage.get(&first_id).unwrap().unwrap().text, "first");
    assert_eq!(storage.get(&second_id).unwrap().unwrap().text, "second");

    drop(storage);
    cleanup_test_files(&files);
}

#[test]
fn compaction_rewrites_live_records_through_temp_record_store() {
    ensure_test_dir();
    let test_path = ".piramid/tests/test_record_store_compact.db";
    let files = vec![
        test_path,
        ".piramid/tests/test_record_store_compact.db.index.db",
        ".piramid/tests/test_record_store_compact.db.wal.db",
        ".piramid/tests/test_record_store_compact.db.vecindex.db",
        ".piramid/tests/test_record_store_compact.db.metadata.db",
        ".piramid/tests/test_record_store_compact.db.wal.meta",
        ".piramid/tests/test_record_store_compact.db.compact",
    ];
    cleanup_test_files(&files);

    let mut storage = Collection::open(test_path).unwrap();
    let keep_id = storage
        .insert(Document::new(vec![1.0, 0.0, 0.0], "keep".to_string()))
        .unwrap();
    let delete_id = storage
        .insert(Document::new(vec![0.0, 1.0, 0.0], "delete".to_string()))
        .unwrap();
    storage.delete(&delete_id).unwrap();

    let stats = compact(&mut storage).unwrap();

    assert_eq!(stats.original_entries, 1);
    assert_eq!(stats.compacted_entries, 1);
    assert_eq!(storage.count(), 1);
    assert_eq!(storage.get(&keep_id).unwrap().unwrap().text, "keep");
    assert!(storage.get(&delete_id).unwrap().is_none());
    assert!(fs::metadata(format!("{}.compact", test_path)).is_err());

    drop(storage);
    cleanup_test_files(&files);
}
