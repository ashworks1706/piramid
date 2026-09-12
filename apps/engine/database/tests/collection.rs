#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]

use std::fs;
use {
    piramid_core::config::CacheConfig,
    piramid_core::config::CollectionConfig,
    piramid_core::config::MemoryConfig,
    piramid_core::metadata::metadata,
    piramid_core::Document,
    piramid_database::search::SearchParams,
    piramid_database::Collection,
    piramid_database::{compact, CollectionOpenOptions},
    piramid_hardware::compute::Metric,
};

fn ensure_test_dir() {
    let _ = fs::create_dir_all(env!("CARGO_TARGET_TMPDIR"));
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
    let test_path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_basic.db");
    let files = vec![
        test_path,
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_basic.db.offsets.db"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_basic.db.wal.db"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_basic.db.vecindex.db"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_basic.db.manifest.db"),
    ];
    cleanup_test_files(&files);

    let mut storage = Collection::open(test_path).unwrap();
    let entry = Document::new(vec![1.0, 2.0, 3.0], "test".to_string());
    let id = storage.insert(entry).unwrap();

    let retrieved = storage.get(&id).unwrap().unwrap();
    assert_eq!(retrieved.text, "test");
    assert_eq!(retrieved.vector(), vec![1.0, 2.0, 3.0]);

    drop(storage);
    cleanup_test_files(&files);
}

#[test]
fn persistence_roundtrip() {
    ensure_test_dir();
    let test_path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_persist.db");
    let files = vec![
        test_path,
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_persist.db.offsets.db"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_persist.db.wal.db"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_persist.db.vecindex.db"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_persist.db.manifest.db"),
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
    let test_path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_search.db");
    let files = vec![
        test_path,
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_search.db.offsets.db"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_search.db.wal.db"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_search.db.vecindex.db"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_search.db.manifest.db"),
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
            .insert(Document::new(vec.clone(), format!("vec{i}")))
            .unwrap();
    }

    let params = SearchParams::default();
    let results = storage
        .search(&[1.0, 0.0, 0.0], 2, Metric::Cosine, params)
        .unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].document.text, "vec0");

    drop(storage);
    cleanup_test_files(&files);
}

#[test]
fn batch_search_multi_queries() {
    ensure_test_dir();
    let test_path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_batch_search.db");
    let files = vec![
        test_path,
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_batch_search.db.offsets.db"
        ),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_batch_search.db.wal.db"),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_batch_search.db.vecindex.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_batch_search.db.manifest.db"
        ),
    ];
    cleanup_test_files(&files);

    let mut storage = Collection::open(test_path).unwrap();
    for i in 0..10 {
        storage
            .insert(Document::new(vec![i as f32, 0.0, 0.0], format!("vec{i}")))
            .unwrap();
    }

    let queries = vec![
        vec![0.0, 0.0, 0.0],
        vec![5.0, 0.0, 0.0],
        vec![9.0, 0.0, 0.0],
    ];
    let results = storage
        .search_batch_with(&queries, 2, Metric::Cosine, SearchParams::default())
        .unwrap();
    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|hits| !hits.is_empty()));

    drop(storage);
    cleanup_test_files(&files);
}

#[test]
fn no_mmap_insert_grows_file_without_panicking() {
    ensure_test_dir();
    let test_path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_no_mmap_grow.db");
    let files = vec![
        test_path,
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_no_mmap_grow.db.offsets.db"
        ),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_no_mmap_grow.db.wal.db"),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_no_mmap_grow.db.wal.meta"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_no_mmap_grow.db.vecindex.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_no_mmap_grow.db.manifest.db"
        ),
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
    assert_eq!(retrieved.vector().len(), vector.len());

    drop(storage);
    cleanup_test_files(&files);
}

#[test]
fn updates_write_one_wal_entry_each() {
    ensure_test_dir();
    let test_path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_update_wal.db");
    let files = vec![
        test_path,
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_update_wal.db.offsets.db"
        ),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_update_wal.db.wal.db"),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_update_wal.db.vecindex.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_update_wal.db.manifest.db"
        ),
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
    assert_eq!(
        storage.get(&id).unwrap().unwrap().vector(),
        vec![3.0, 2.0, 1.0]
    );

    let wal = fs::read_to_string(format!("{test_path}.wal.db")).unwrap();
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
fn update_vector_persists_new_raw_vector_after_reopen() {
    ensure_test_dir();
    let test_path = concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/test_update_vector_persist.db"
    );
    let files = vec![
        test_path,
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_update_vector_persist.db.offsets.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_update_vector_persist.db.wal.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_update_vector_persist.db.vecindex.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_update_vector_persist.db.manifest.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_update_vector_persist.db.wal.meta"
        ),
    ];
    cleanup_test_files(&files);

    let id = {
        let mut storage = Collection::open(test_path).unwrap();
        let id = storage
            .insert(Document::new(vec![1.0, 2.0, 3.0], "updated".into()))
            .unwrap();
        storage.update_vector(&id, vec![3.0, 2.0, 1.0]).unwrap();
        id
    };

    let storage = Collection::open(test_path).unwrap();
    assert_eq!(
        storage.get(&id).unwrap().unwrap().vector(),
        vec![3.0, 2.0, 1.0]
    );

    drop(storage);
    cleanup_test_files(&files);
}

#[test]
fn sidecar_files_persist_at_checkpoint_only() {
    ensure_test_dir();
    let test_path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_checkpoint_only.db");
    let files = vec![
        test_path,
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_checkpoint_only.db.offsets.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_checkpoint_only.db.wal.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_checkpoint_only.db.vecindex.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_checkpoint_only.db.manifest.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_checkpoint_only.db.wal.meta"
        ),
    ];
    cleanup_test_files(&files);

    let mut storage = Collection::open(test_path).unwrap();
    storage
        .insert(Document::new(vec![1.0, 2.0, 3.0], "checkpoint only".into()))
        .unwrap();

    assert!(fs::metadata(format!("{test_path}.offsets.db")).is_err());
    assert!(fs::metadata(format!("{test_path}.vecindex.db")).is_err());

    storage.checkpoint().unwrap();

    assert!(fs::metadata(format!("{test_path}.offsets.db")).is_ok());
    assert!(fs::metadata(format!("{test_path}.vecindex.db")).is_ok());

    drop(storage);
    cleanup_test_files(&files);
}

#[test]
fn metadata_cache_is_bounded_without_evicting_vectors() {
    ensure_test_dir();
    let test_path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_cache_manager_bounds.db");
    let files = vec![
        test_path,
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_cache_manager_bounds.db.offsets.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_cache_manager_bounds.db.wal.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_cache_manager_bounds.db.vecindex.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_cache_manager_bounds.db.manifest.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_cache_manager_bounds.db.wal.meta"
        ),
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

    assert_eq!(storage.vector_reader().len(), 2);
    assert_eq!(storage.metadata_view().len(), 1);
    assert!(storage.vector_reader().get(&id_a).is_some());
    assert!(storage.vector_reader().get(&id_b).is_some());

    drop(storage);
    cleanup_test_files(&files);
}

#[test]
fn append_cursor_survives_reopen_and_preserves_existing_records() {
    ensure_test_dir();
    let test_path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_append_cursor_reopen.db");
    let files = vec![
        test_path,
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_append_cursor_reopen.db.offsets.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_append_cursor_reopen.db.wal.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_append_cursor_reopen.db.vecindex.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_append_cursor_reopen.db.manifest.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_append_cursor_reopen.db.wal.meta"
        ),
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
    let test_path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_record_store_compact.db");
    let files = vec![
        test_path,
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_record_store_compact.db.offsets.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_record_store_compact.db.wal.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_record_store_compact.db.vecindex.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_record_store_compact.db.manifest.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_record_store_compact.db.wal.meta"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_record_store_compact.db.compact"
        ),
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
    assert!(fs::metadata(format!("{test_path}.compact")).is_err());

    drop(storage);
    cleanup_test_files(&files);
}

// A collection with sync_on_write enabled accepts writes and replays them on reopen.
#[test]
fn writes_replay_with_sync_on_write_enabled() {
    use piramid_core::config::CollectionConfig;

    let path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_wal_sync.db");
    for suffix in [
        "",
        ".offsets.db",
        ".wal.db",
        ".vecindex.db",
        ".manifest.db",
        ".wal.meta",
    ] {
        let _ = fs::remove_file(format!("{path}{suffix}"));
    }

    let mut config = CollectionConfig::default();
    config.wal.sync_on_write = true;
    // High enough that no checkpoint fires; the WAL must carry the writes on its own.
    config.wal.checkpoint_frequency = 10_000;

    {
        let mut collection = Collection::open_with_options(path, config.clone().into()).unwrap();
        collection
            .insert(Document::new(vec![1.0, 0.0], "durable".to_string()))
            .unwrap();
    }

    let reopened = Collection::open_with_options(path, config.into()).unwrap();
    assert_eq!(
        reopened.count(),
        1,
        "the synced write must replay from the WAL"
    );
}

// A WAL that grows past max_log_size triggers a checkpoint.
#[test]
fn a_wal_past_max_log_size_triggers_a_checkpoint() {
    use piramid_core::config::CollectionConfig;

    let path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_wal_max_size.db");
    for suffix in [
        "",
        ".offsets.db",
        ".wal.db",
        ".vecindex.db",
        ".manifest.db",
        ".wal.meta",
    ] {
        let _ = fs::remove_file(format!("{path}{suffix}"));
    }

    let mut config = CollectionConfig::default();
    config.wal.checkpoint_frequency = 10_000; // never fires on count
    config.wal.max_log_size = 256; // a few entries' worth

    let mut collection = Collection::open_with_options(path, config.into()).unwrap();
    for i in 0..12 {
        collection
            .insert(Document::new(vec![i as f32, 0.0], format!("doc{i}")))
            .unwrap();
    }

    // A checkpoint rotates the log, so it cannot still be above the bound.
    let wal_len = fs::metadata(format!("{path}.wal.db")).unwrap().len();
    assert!(
        wal_len < 256 * 4,
        "the size trigger never fired; WAL is {wal_len} bytes"
    );
    assert_eq!(collection.count(), 12);
}

#[test]
fn a_collection_hands_its_vectors_over_as_one_slab() {
    ensure_test_dir();
    let test_path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_slab.db");
    let files = vec![
        test_path,
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_slab.db.offsets.db"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_slab.db.wal.db"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_slab.db.vecindex.db"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_slab.db.manifest.db"),
    ];
    cleanup_test_files(&files);

    let mut storage = Collection::open(test_path).unwrap();

    let ids: Vec<_> = (0..4)
        .map(|i| {
            storage
                .insert(Document::new(vec![i as f32, 1.0, 2.0], format!("doc {i}")))
                .unwrap()
        })
        .collect();

    // The reader is contiguous, so a batch kernel takes the whole candidate set in one copy.
    let slab = storage
        .vector_reader()
        .as_slab()
        .expect("a collection with no deletes is contiguous");
    assert_eq!(slab.dim, 3);
    assert_eq!(slab.data.len(), 4 * 3);
    assert_eq!(slab.rows(), 4);

    // A delete withdraws the slab fast path until an insert reuses the hole.
    storage.delete(&ids[1]).unwrap();
    assert!(storage.vector_reader().as_slab().is_none());

    storage
        .insert(Document::new(vec![9.0, 9.0, 9.0], "refill".to_string()))
        .unwrap();
    let slab = storage
        .vector_reader()
        .as_slab()
        .expect("the hole was reused, so the slab is whole again");
    assert_eq!(slab.data.len(), 4 * 3);

    drop(storage);
    cleanup_test_files(&files);
}

// The interval trigger counts from the open, so it fires before any other trigger has run a
// first checkpoint.
#[test]
fn the_checkpoint_interval_runs_from_the_open() {
    use piramid_core::config::CollectionConfig;

    let path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_wal_interval.db");
    for suffix in [
        "",
        ".offsets.db",
        ".wal.db",
        ".vecindex.db",
        ".manifest.db",
        ".wal.meta",
    ] {
        let _ = fs::remove_file(format!("{path}{suffix}"));
    }

    let mut config = CollectionConfig::default();
    config.wal.checkpoint_frequency = 10_000;
    config.wal.checkpoint_interval_secs = Some(0);

    let mut collection = Collection::open_with_options(path, config.into()).unwrap();
    assert_eq!(collection.checkpoint.last_checkpoint(), None);
    collection
        .insert(Document::new(vec![1.0, 0.0], "doc".to_string()))
        .unwrap();
    assert!(collection.checkpoint.last_checkpoint().is_some());
}

// An auto index moves to the family its thresholds name as the collection grows, and every
// vector stays searchable across each move.
#[test]
fn an_auto_index_grows_into_the_family_its_size_picks() {
    use piramid_core::config::{AutoIndexConfig, CollectionConfig, IndexConfig};
    use piramid_database::index::IndexType;
    use piramid_database::search::SearchParams;
    use piramid_hardware::compute::Metric;

    let path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_auto_index_growth.db");
    for suffix in [
        "",
        ".offsets.db",
        ".wal.db",
        ".vecindex.db",
        ".manifest.db",
        ".wal.meta",
    ] {
        let _ = fs::remove_file(format!("{path}{suffix}"));
    }
    let config = CollectionConfig {
        index: IndexConfig::Auto {
            metric: Metric::Cosine,
            auto: AutoIndexConfig {
                flat_max_vectors: 5,
                ivf_max_vectors: 10,
                ..AutoIndexConfig::default()
            },
        },
        ..CollectionConfig::default()
    };
    let mut collection = Collection::open_with_options(path, config.into()).unwrap();
    let vector = |i: usize| {
        let angle = i as f32 * 0.4;
        vec![angle.cos(), angle.sin(), 0.1 * i as f32]
    };

    let mut families = Vec::new();
    for i in 0..12 {
        collection
            .insert(Document::new(vector(i), format!("doc{i}")))
            .unwrap();
        families.push(collection.vector_index().index_type());
    }
    assert_eq!(families[3], IndexType::Flat);
    assert_eq!(
        families[4],
        IndexType::Ivf,
        "the fifth vector reaches flat_max_vectors"
    );
    assert_eq!(
        families[9],
        IndexType::Hnsw,
        "the tenth vector reaches ivf_max_vectors"
    );

    let hits = collection
        .search(&vector(7), 1, Metric::Cosine, SearchParams::default())
        .unwrap();
    assert_eq!(hits[0].document.text, "doc7");
}
